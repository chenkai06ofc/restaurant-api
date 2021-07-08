use hyper::service::{make_service_fn, service_fn};
use hyper::{Request, Body, Response, StatusCode, Method, Server};
use std::collections::HashMap;
use redis::{Commands, Connection};
use rand::Rng;

static mut cur_item_no: u64 = 1;

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
    println!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}


fn parse_query_str(query_string: &str) -> HashMap<String, String> {
    let mut params: HashMap<String, String> = HashMap::new();
    let mut p = &mut params;
    query_string.split("&").for_each(|s| {
        let (key, value) = s.split_once("=").unwrap();
        p.insert(String::from(key), String::from(value));
    });
    return params;
}

async fn handle_req(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let query_string = req.uri().query().unwrap();
    let mut params = parse_query_str(query_string);

    // get Redis connection
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/item/add") => {
            match (params.get("table_no"), params.get("content")) {
                (Some(table_no), Some(content)) => unsafe {
                    let item_no = next_item_no().to_string();

                    // HashMap table:{table_no}
                    let table_key = format!("table:{}", table_no);
                    let _ : () = con.hset(&table_key, &item_no, content).unwrap();

                    // HashMap item:{table_no-item_no}
                    let item_key = format!("item:{}-{}", table_no, item_no);
                    let prepare_time_min = rand::thread_rng().gen_range(5..15);
                    let time_str = prepare_time_min.to_string();
                    let _ : () = con.hset_multiple(
                        &item_key,
                        &[("content", content), ("prepare_time_min", &time_str) ])
                        .unwrap();

                    Ok(Response::new("add succeed".into()))
                }
                _ => Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Please specify table_no and content"))
                    .unwrap())
            }
        }
        (&Method::POST, "/item/remove") => {
            match (params.get("table_no"), params.get("item_no")) {
                (Some(table_no), Some(item_no)) => {
                    let mut pipe = redis::pipe();
                    let table_key = format!("table:{}", table_no);
                    let item_key = format!("item:{}-{}", table_no, item_no);
                    let _ : () = pipe.hdel(&table_key, item_no).del(&item_key).query(&mut con).unwrap();
                    Ok(Response::new("remove succeed".into()))
                }
                _ => Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .body(Body::from("Please specify table_no and content"))
                    .unwrap())
            }
        }
        (&Method::GET, "/item/query") => {
            Ok(Response::new("query succeed".into()))
        }
        _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap())
    }
}
