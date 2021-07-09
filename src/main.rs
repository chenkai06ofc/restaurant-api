use hyper::service::{make_service_fn, service_fn};
use hyper::{Request, Body, Response, StatusCode, Method, Server};
use std::collections::HashMap;
use redis::{Commands, Connection};
use rand::Rng;
use serde::{Serialize, Deserialize};
use tokio::time::{self, Duration};

mod util;

static mut cur_item_no: u64 = 1;
const COOK_QUEUE_LEN: u32 = 20;

unsafe fn next_item_no() -> u64{
    let item_no = cur_item_no;
    cur_item_no+=1;
    item_no
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3000).into();
    let server = Server::bind(&addr).serve(make_service_fn(|_| async {
        Ok::<_, hyper::Error>(service_fn(handle_req))
    }));
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();
    let _ : () = con.set("cook_queue_ptr", 0).unwrap();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            cook_complete(&mut con);
        }
    });

    println!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}


async fn handle_req(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let query_string = req.uri().query().unwrap();
    let mut params = util::parse_query_str(query_string);

    // get Redis connection
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/item/add") => {
            match (params.get("table_no"), params.get("content")) {
                (Some(table_no), Some(content)) => unsafe {
                    add_item(&mut con, table_no, content);
                    Ok(Response::new("add succeed".into()))
                }
                _ => Ok(bad_request("Please specify table_no & content"))
            }
        }
        (&Method::POST, "/item/remove") => {
            match (params.get("table_no"), params.get("item_no")) {
                (Some(table_no), Some(item_no)) => {
                    remove_item(&mut con, table_no, item_no);
                    Ok(Response::new("remove succeed".into()))
                }
                _ => Ok(bad_request("Please specify table_no & item_no"))
            }
        }
        (&Method::GET, "/item/query") => {
            match (params.get("table_no"), params.get("item_no")) {
                (Some(table_no), Some(item_no)) => {
                    let item = get_item(&mut con, table_no, item_no);
                    let serialized = serde_json::to_string(&item).unwrap();
                    Ok(Response::new(serialized.into()))
                }
                (Some(table_no), None) => {
                    let map : HashMap<String, String> = con.hgetall(format!("table:{}", table_no)).unwrap();
                    let serialized = serde_json::to_string(&map).unwrap();
                    Ok(Response::new(serialized.into()))
                }
                _ => Ok(not_found())
            }

        }
        _ => Ok(not_found())
    }
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::empty())
        .unwrap()
}

fn bad_request(msg: &'static str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::from(msg))
        .unwrap()
}

unsafe fn add_item(con: &mut Connection, table_no: &String, content: &String) {
    let item_no = next_item_no().to_string();

    // update redis HashMap table:{table_no}
    let table_key = format!("table:{}", table_no);
    let _ : () = con.hset(&table_key, &item_no, content).unwrap();

    // update redis HashMap item:{table_no-item_no}
    let item_key = format!("item:{}-{}", table_no, item_no);
    let prepare_time_min = rand::thread_rng().gen_range(5..15);
    println!("  add {}, time: {}", &item_key, prepare_time_min);
    let time_str = prepare_time_min.to_string();
    let _ : () = con.hset_multiple(
        &item_key,
        &[("content", content), ("prepare_time_min", &time_str) ])
        .unwrap();

    // update redis Set cook_queue:{min}
    let mut cook_queue_ptr: u32 = con.get("cook_queue_ptr").unwrap();
    cook_queue_ptr = (cook_queue_ptr + prepare_time_min) % COOK_QUEUE_LEN;
    let _ : () = con.sadd(format!("cook_queue:{}", cook_queue_ptr), format!("{}-{}",table_no, item_no)).unwrap();
}

fn get_item(con: &mut Connection, table_no: &String, item_no: &String) -> Item {
    let map : HashMap<String, String> =
        con.hgetall(format!("item:{}-{}", table_no, item_no)).unwrap();

    Item {
        table_no: table_no.parse().unwrap(),
        item_no: item_no.parse().unwrap(),
        content: String::from(map.get("content").unwrap()),
        prepare_time_min: map.get("prepare_time_min").unwrap().parse().unwrap()
    }
}

fn remove_item(con: &mut Connection, table_no: &String, item_no: &String) {
    let mut pipe = redis::pipe();
    let table_key = format!("table:{}", table_no);
    let item_key = format!("item:{}-{}", table_no, item_no);
    let _ : () = pipe.hdel(&table_key, item_no).del(&item_key).query(con).unwrap();
}

fn cook_complete(con: &mut Connection) {
    let mut cook_queue_ptr: u32 = con.get("cook_queue_ptr").unwrap();
    let key = format!("cook_queue:{}", cook_queue_ptr);
    let vec: Vec<String> = con.smembers(&key).unwrap();
    println!("round {} cooked: {}", cook_queue_ptr, vec.join(" "));
    let mut pipe = redis::pipe();
    let mut p = &mut pipe;
    for s in vec.iter() {
        let (table_no, item_no) = s.split_once("-").unwrap();
        p = p.hdel(format!("table:{}", table_no), item_no).del(format!("item:{}-{}", table_no, item_no));
    }
    cook_queue_ptr = (cook_queue_ptr + 1) % COOK_QUEUE_LEN;
    let _ : () = p.del(&key).set("cook_queue_ptr", cook_queue_ptr).query(con).unwrap();
}

#[derive(Serialize, Deserialize)]
struct Item {
    table_no: u32,
    item_no: u32,
    content: String,
    //generate_time:
    prepare_time_min: u32,
}
