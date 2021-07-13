use serde::{Serialize, Deserialize};
use redis::{Commands, Connection};
use std::collections::{HashMap, HashSet};
use rand::Rng;
use tokio::sync::Mutex;
use std::sync::Arc;
use mysql::Pool;
use mysql::prelude::Queryable;

pub const MIN_TABLE_NO: u32 = 1;
pub const MAX_TABLE_NO: u32 = 100;

const COOK_QUEUE_LEN: u32 = 20;

// redis keys
pub const COOK_QUEUE_PTR: &str = "cook_queue_ptr";
pub const NEXT_ITEM_NO: &str = "next_item_no";
pub const REMOVED_ITEMS: &str = "removed_items";

#[derive(Serialize, Deserialize)]
pub struct Item {
    table_no: u32,
    item_no: u64,
    content: String,
    create_at: u64, // in seconds
    time_take: u32, // in minutes
}

impl Item {
    pub fn new(table_no: u32, item_no: u64, content: String, create_at: u64, time_take: u32) -> Item {
        Item {
            table_no,
            item_no,
            content,
            create_at,
            time_take
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct AddReq {
    pub table_no: u32,
    pub content: String
}

impl AddReq {
    pub fn from (s: &str) -> serde_json::error::Result<AddReq> {
        serde_json::from_str(s)
    }
}

#[derive(Serialize, Deserialize)]
pub struct RemoveReq {
    pub table_no: u32,
    pub item_no: u64
}

impl RemoveReq {
    pub fn from (s: &str) -> serde_json::error::Result<RemoveReq> {
        serde_json::from_str(s)
    }
}

fn table_key(table_no: u32) -> String {
    format!("table:{}", table_no)
}

fn item_l_key(table_no: u32, item_no: u64) -> String {
    format!("item:{}-{}", table_no, item_no)
}

fn item_s_key(table_no: u32, item_no: u64) -> String {
    format!("{}-{}", table_no, item_no)
}

fn cook_queue_key(cook_queue_ptr: u32) -> String {
    format!("cook_queue:{}", cook_queue_ptr)
}

pub async fn get_item(r_con_hold: Arc<Mutex<Connection>>,
                      table_no: u32,
                      item_no: u64) -> Result<Item, String> {
    let mut r_con = r_con_hold.lock().await;
    let map : HashMap<String, String> = r_con.hgetall(item_l_key(table_no, item_no)).unwrap();
    if map.is_empty() {
        Result::Err(format!("Item:{}-{} does not exist", table_no, item_no))
    } else {
        Result::Ok(Item::new(
            table_no,
            item_no,
            String::from(map.get("content").unwrap()),
            map.get("create_at").unwrap().parse().unwrap(),
            map.get("time_take").unwrap().parse().unwrap()
        ))
    }
}

pub async fn add_item(r_con_hold: Arc<Mutex<Connection>>,
                      pool_hold: Arc<Pool>,
                      table_no: u32,
                      content: &String) -> Result<String, String> {
    if table_no < MIN_TABLE_NO || table_no > MAX_TABLE_NO {
        return Result::Err(format!("Please specify table_no between {} and {}", MIN_TABLE_NO, MAX_TABLE_NO));
    }

    // prepare data
    let mut item_no = {
        let mut r_con = r_con_hold.lock().await;
        r_con.incr(NEXT_ITEM_NO, 1).unwrap()
    };
    item_no -= 1;
    let create_at = crate::util::secs_since_epoch();
    let time_take = rand::thread_rng().gen_range(5..15);

    // update MySQL
    let mut conn = pool_hold.get_conn().unwrap();
    conn.exec_drop("INSERT INTO item_t (status, table_no, item_no, content, create_at, time_take) VALUES (?, ?, ?, ?, ?, ?)",
                 ("cooking", table_no, item_no, content, create_at, time_take)).unwrap();

    // update Redis
    let mut r_con = r_con_hold.lock().await;
    let table_key = table_key(table_no);
    let item_key = item_l_key(table_no, item_no);
    println!("  add {}, time: {} mins", &item_key, time_take);
    let time_str = time_take.to_string();

    let mut cook_queue_ptr: u32 = r_con.get(COOK_QUEUE_PTR).unwrap();
    cook_queue_ptr = (cook_queue_ptr + time_take) % COOK_QUEUE_LEN;

    let _ : () = redis::pipe()
        .hset(&table_key, &item_no.to_string(), content)
        .hset_multiple(&item_key, &[("content", content), ("create_at", &create_at.to_string()), ("time_take", &time_str) ])
        .sadd(cook_queue_key(cook_queue_ptr), item_s_key(table_no, item_no))
        .query(&mut (*r_con)).unwrap();
    Result::Ok(format!("OK"))
}

pub async fn remove_item(r_con_hold: Arc<Mutex<Connection>>,
                         pool_hold: Arc<Pool>,
                         table_no: u32,
                         item_no: u64) -> Result<String, String> {
    let item_map: HashMap<String, String> = {
        let mut r_con = r_con_hold.lock().await;
        r_con.hgetall(item_l_key(table_no, item_no)).unwrap()
    };

    if item_map.is_empty() {
        return Result::Err(format!("Item:{}-{} does not exist", table_no, item_no));
    }
    let create_at: u64 = item_map.get("create_at").unwrap().parse().unwrap();
    let time_take: u64 = item_map.get("time_take").unwrap().parse().unwrap();

    if crate::util::secs_since_epoch() > (create_at + (time_take - 3) * crate::config::seconds_per_min()) {
        return Result::Err(format!("Cannot remove item that will be completed in less than 3 minutes."));
    }

    println!("  remove item: {}-{}", table_no, item_no);
    // update MySQL
    let mut conn = pool_hold.get_conn().unwrap();
    conn.exec_drop("UPDATE item_t SET status='canceled' where status=? and table_no=? and item_no=?",
                   ("cooking", table_no, item_no)).unwrap();

    let table_key = table_key(table_no);
    // update Redis
    let mut r_con = r_con_hold.lock().await;

    let _ : () = redis::pipe()
        .hdel(&table_key, item_no)
        .del(item_l_key(table_no, item_no))
        .sadd(REMOVED_ITEMS, item_s_key(table_no, item_no))
        .query(&mut (*r_con)).unwrap();
    Result::Ok(format!("OK"))
}

pub async fn cook_complete(r_con_hold: Arc<Mutex<Connection>>, pool_hold: Arc<Pool>) {
    let (mut cook_queue_ptr, key, cooked_items, removed_items) = {
        let mut r_con = r_con_hold.lock().await;
        let cook_queue_ptr: u32 = r_con.get(COOK_QUEUE_PTR).unwrap();
        let key = cook_queue_key(cook_queue_ptr);
        let cooked_items: Vec<String> = r_con.smembers(&key).unwrap();
        let removed_items: HashSet<String> = r_con.smembers(REMOVED_ITEMS).unwrap();
        (cook_queue_ptr, key, cooked_items, removed_items)
    };


    let mut pipe = &mut redis::pipe();

    print!("{}th minute cooked: ", cook_queue_ptr);
    for s in cooked_items.iter() {
        if removed_items.contains(s) {
            pipe = pipe.srem(REMOVED_ITEMS, s);
        } else {
            print!("{} ", s);
            let (table_no, item_no) = s.split_once("-").unwrap();
            let table_no: u32 = table_no.parse().unwrap();
            let item_no: u64 = item_no.parse().unwrap();

            // update MySQL
            let mut conn = pool_hold.get_conn().unwrap();
            conn.exec_drop("UPDATE item_t SET status='cooked' where status=? and table_no=? and item_no=?",
                           ("cooking", table_no, item_no)).unwrap();

            // update Redis
            pipe = pipe
                .hdel(table_key(table_no), item_no)
                .del(item_l_key(table_no, item_no));
        }
    }
    println!();
    cook_queue_ptr = (cook_queue_ptr + 1) % COOK_QUEUE_LEN;
    let mut r_con = r_con_hold.lock().await;
    let _ : () = pipe
        .del(&key)
        .set(COOK_QUEUE_PTR, cook_queue_ptr)
        .query(&mut (*r_con)).unwrap();
}