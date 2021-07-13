use hyper::service::{make_service_fn, service_fn};
use hyper::{Request, Body, Response, StatusCode, Method, Server};
use std::collections::HashMap;
use redis::{Commands, Connection};
use tokio::time::{self, Duration};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::str;

mod util;
mod item;
mod config;

use item::{AddReq, RemoveReq};
use mysql::Pool;

const REMOVE_WRONG_PARAM: &str = r#"[parameters wrong]
Please specify (table_no, item_no) as positive integer.
Like {table_no: 1, item_no: 4}"#;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = redis::Client::open(&(config::redis_url())[..]).unwrap();
    let mut r_con = client.get_connection().unwrap();
    let _ : () = r_con.set(item::COOK_QUEUE_PTR, 0).unwrap();
    let _ : () = r_con.set(item::NEXT_ITEM_NO, 1).unwrap();

    // redis connection
    let r_con_hold = Arc::new(Mutex::new(r_con));
    let r_con_hold1 = r_con_hold.clone();
    let r_con_hold2 = r_con_hold.clone();

    // mysql connection pool
    let pool = wait_get_mysql_conn().await;
    let pool_hold = Arc::new(pool);
    let pool_hold1 = pool_hold.clone();
    let pool_hold2 = pool_hold.clone();

    let make_service = make_service_fn(move |_| {
        let r_con_hold = r_con_hold1.clone();
        let pool_hold = pool_hold1.clone();
        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| handle_req(r_con_hold.clone(), pool_hold.clone(), req)))
        }
    });

    let addr = ([0, 0, 0, 0], 3000).into();
    let server = Server::bind(&addr).serve(make_service);

    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(config::seconds_per_min()));
        loop {
            interval.tick().await;
            item::cook_complete(r_con_hold2.clone(), pool_hold2.clone()).await;
        }
    });

    println!("Listening on http://{}\nYou can start to interact with restaurant-api\n", addr);
    server.await?;
    Ok(())
}


async fn handle_req(r_con_hold: Arc<Mutex<Connection>>,
                    pool_hold: Arc<Pool>,
                    req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/item/add") => handle_add(r_con_hold, pool_hold, req).await,
        (&Method::POST, "/item/remove") => handle_remove(r_con_hold, pool_hold, req).await,
        (&Method::GET, "/item/query") => {
            let query_string = match req.uri().query() {
                Some(str) => str,
                None => return Ok(not_found("parameter (table_no & [item_no]) not found"))
            };
            let params = util::parse_query_str(query_string);
            handle_query(r_con_hold, params).await
        }
        _ => Ok(not_found("path not found"))
    }
}

async fn wait_get_mysql_conn() -> Pool {
    let mut interval = time::interval(Duration::from_secs(5));
    let mut result;
    loop {
        result = Pool::new(config::mysql_opts());
        if result.is_ok() {
            break;
        } else {
            println!("Waiting for MySQL to become available...");
            interval.tick().await;
        }
    }
    result.unwrap()
}

fn not_found(msg: &'static str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from(format!("Error: {}", msg)))
        .unwrap()
}

fn bad_request(msg: &'static str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .body(Body::from(msg))
        .unwrap()
}

fn op_fail(msg: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::EXPECTATION_FAILED)
        .body(Body::from(format!("[Operation Failed]\n{}", msg)))
        .unwrap()
}


async fn handle_add(r_con_hold: Arc<Mutex<Connection>>,
                    pool_hold: Arc<Pool>,
                    req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let body_u8 = get_u8_body(req).await;
    let s = str::from_utf8(&body_u8).unwrap();
    match AddReq::from(s) {
        Ok(add_req) => {
            let re = item::add_item(r_con_hold, pool_hold, add_req.table_no, &add_req.content).await;
            match re {
                Ok(_) => Ok(Response::new("item added".into())),
                Err(s) => Ok(op_fail(s))
            }
        }
        Err(_) => Ok(bad_request("Please specify table_no & content"))
    }
}


async fn handle_remove(r_con_hold: Arc<Mutex<Connection>>,
                       pool_hold: Arc<Pool>,
                       req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let body_u8 = get_u8_body(req).await;
    let s = str::from_utf8(&body_u8).unwrap();
    match RemoveReq::from(s) {
        Ok(remove_req) => {
            let re = item::remove_item(r_con_hold, pool_hold, remove_req.table_no, remove_req.item_no).await;
            match re {
                Ok(_) => Ok(Response::new("item removed".into())),
                Err(s) => Ok(op_fail(s))
            }
        }
        Err(_) => Ok(bad_request(REMOVE_WRONG_PARAM))
    }
}


async fn handle_query(r_con_hold: Arc<Mutex<Connection>>,
                      params: HashMap<String, String>) -> Result<Response<Body>, hyper::Error> {
    match (params.get("table_no"), params.get("item_no")) {
        (Some(table_no), Some(item_no)) => {
            let table_no: u32 = table_no.parse().unwrap();
            let item_no: u64 = item_no.parse().unwrap();
            let item = item::get_item(r_con_hold, table_no, item_no).await;
            let serialized = serde_json::to_string(&item).unwrap();
            Ok(Response::new(serialized.into()))
        }
        (Some(table_no), None) => {
            let mut r_con = r_con_hold.lock().await;
            let map : HashMap<String, String> = r_con.hgetall(format!("table:{}", table_no)).unwrap();
            let serialized = serde_json::to_string(&map).unwrap();
            Ok(Response::new(serialized.into()))
        }
        _ => Ok(not_found("parameter (table_no & [item_no]) not found"))
    }
}

async fn get_u8_body(req: Request<Body>) -> Vec<u8> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    whole_body.iter().cloned().collect::<Vec<u8>>()
}