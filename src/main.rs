use hyper::service::{make_service_fn, service_fn};
use hyper::{Request, Body, Response, StatusCode, Method, Server};
use std::collections::HashMap;
use redis::{Commands, Connection};
use serde::{Serialize, Deserialize};
use tokio::time::{self, Duration};

mod util;
mod item;

use item::Item;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3000).into();
    let server = Server::bind(&addr).serve(make_service_fn(|_| async {
        Ok::<_, hyper::Error>(service_fn(handle_req))
    }));
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let mut con = client.get_connection().unwrap();
    let _ : () = con.set(item::COOK_QUEUE_PTR, 0).unwrap();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            item::cook_complete(&mut con);
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
                    let table_no: u32 = table_no.parse().unwrap();
                    item::add_item(&mut con, table_no, content);
                    Ok(Response::new("add succeed".into()))
                }
                _ => Ok(bad_request("Please specify table_no & content"))
            }
        }
        (&Method::POST, "/item/remove") => {
            match (params.get("table_no"), params.get("item_no")) {
                (Some(table_no), Some(item_no)) => {
                    let table_no: u32 = table_no.parse().unwrap();
                    let item_no: u64 = item_no.parse().unwrap();
                    item::remove_item(&mut con, table_no, item_no);
                    Ok(Response::new("remove succeed".into()))
                }
                _ => Ok(bad_request("Please specify table_no & item_no"))
            }
        }
        (&Method::GET, "/item/query") => {
            match (params.get("table_no"), params.get("item_no")) {
                (Some(table_no), Some(item_no)) => {
                    let table_no: u32 = table_no.parse().unwrap();
                    let item_no: u64 = item_no.parse().unwrap();
                    let item = item::get_item(&mut con, table_no, item_no);
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
