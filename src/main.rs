use hyper::service::{make_service_fn, service_fn};
use hyper::{Request, Body, Response, StatusCode, Method, Server};
use std::collections::HashMap;
use redis::{Commands, Connection};
use serde::{Serialize, Deserialize};
use tokio::time::{self, Duration};
use tokio::sync::Mutex;
use std::sync::Arc;
use std::str;

mod util;
mod item;
mod config;

use item::{Item, AddReq, RemoveReq};
use serde_json::Error;

const SECS_PER_MIN: u64 = 20;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    let client = redis::Client::open(&(config::redis_url())[..]).unwrap();
    let mut r_con = client.get_connection().unwrap();
    let _ : () = r_con.set(item::COOK_QUEUE_PTR, 0).unwrap();
    let _ : () = r_con.set(item::NEXT_ITEM_NO, 1).unwrap();

    let r_con_hold = Arc::new(Mutex::new(r_con));
    let r_con_hold1 = r_con_hold.clone();
    let r_con_hold2 = r_con_hold.clone();

    let make_service = make_service_fn(move |_| {
        let r_con_hold = r_con_hold1.clone();
        async move { Ok::<_, hyper::Error>(service_fn(move |req| handle_req(r_con_hold.clone(), req))) }
    });

    let addr = ([0, 0, 0, 0], 3000).into();
    let server = Server::bind(&addr).serve(make_service);

    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(SECS_PER_MIN));
        loop {
            interval.tick().await;
            item::cook_complete(r_con_hold2.clone()).await;
        }
    });

    println!("Listening on http://{}", addr);
    server.await?;
    Ok(())
}


async fn handle_req(r_con_hold: Arc<Mutex<Connection>>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {

    match (req.method(), req.uri().path()) {
        (&Method::POST, "/item/add") => {
            handle_add(r_con_hold, req).await
        }
        (&Method::POST, "/item/remove") => {
            handle_remove(r_con_hold, req).await
        }
        (&Method::GET, "/item/query") => {
            let query_string = req.uri().query().unwrap();
            let mut params = util::parse_query_str(query_string);

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

async fn handle_add(r_con_hold: Arc<Mutex<Connection>>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let body_u8 = get_u8_body(req).await;
    let s = str::from_utf8(&body_u8).unwrap();
    match AddReq::from(s) {
        Ok(add_req) => {
            item::add_item(r_con_hold, add_req.table_no, &add_req.content).await;
            Ok(Response::new("add succeed".into()))
        }
        Err(_) => Ok(bad_request("Please specify table_no & content"))
    }
}


async fn handle_remove(r_con_hold: Arc<Mutex<Connection>>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let body_u8 = get_u8_body(req).await;
    let s = str::from_utf8(&body_u8).unwrap();
    match RemoveReq::from(s) {
        Ok(remove_req) => {
            item::remove_item(r_con_hold, remove_req.table_no, remove_req.item_no).await;
            Ok(Response::new("remove succeed".into()))
        }
        Err(_) => Ok(bad_request("Please specify table_no & item_no"))
    }
}

async fn get_u8_body(req: Request<Body>) -> Vec<u8> {
    let whole_body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    whole_body.iter().cloned().collect::<Vec<u8>>()
}