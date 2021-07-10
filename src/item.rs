use serde::{Serialize, Deserialize};
use redis::{Commands, Connection};
use std::collections::{HashMap, HashSet};
use rand::Rng;
use tokio::sync::Mutex;
use std::sync::Arc;

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
    //generate_time:
    prepare_time_min: u32,
}

impl Item {
    pub fn new(table_no: u32, item_no: u64, content: String, prepare_time_min: u32) -> Item {
        Item {
            table_no,
            item_no,
            content,
            prepare_time_min
        }
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

pub async fn get_item(r_con_hold: Arc<Mutex<Connection>>, table_no: u32, item_no: u64) -> Item {
    let mut r_con = r_con_hold.lock().await;
    let map : HashMap<String, String> = r_con.hgetall(item_l_key(table_no, item_no)).unwrap();

    Item::new(
        table_no,
        item_no,
        String::from(map.get("content").unwrap()),
        map.get("prepare_time_min").unwrap().parse().unwrap()
    )
}

pub async fn add_item(r_con_hold: Arc<Mutex<Connection>>, table_no: u32, content: &String) {
    let mut r_con = r_con_hold.lock().await;

    let mut item_no = r_con.incr(NEXT_ITEM_NO, 1).unwrap();
    item_no -= 1;

    let table_key = table_key(table_no);
    let item_key = item_l_key(table_no, item_no);
    let prepare_time_min = rand::thread_rng().gen_range(5..15);
    println!("  add {}, time: {}", &item_key, prepare_time_min);
    let time_str = prepare_time_min.to_string();


    let mut cook_queue_ptr: u32 = r_con.get(COOK_QUEUE_PTR).unwrap();
    cook_queue_ptr = (cook_queue_ptr + prepare_time_min) % COOK_QUEUE_LEN;

    let _ : () = redis::pipe()
        .hset(&table_key, &item_no.to_string(), content)
        .hset_multiple(&item_key, &[("content", content), ("prepare_time_min", &time_str) ])
        .sadd(cook_queue_key(cook_queue_ptr), item_s_key(table_no, item_no))
        .query(&mut (*r_con)).unwrap();
}

pub async fn remove_item(r_con_hold: Arc<Mutex<Connection>>, table_no: u32, item_no: u64) {
    println!("  remove item: {}-{}", table_no, item_no);
    let table_key = table_key(table_no);

    let mut r_con = r_con_hold.lock().await;
    let _ : () = redis::pipe()
        .hdel(&table_key, item_no)
        .del(item_l_key(table_no, item_no))
        .sadd(REMOVED_ITEMS, item_s_key(table_no, item_no))
        .query(&mut (*r_con)).unwrap();
}

pub async fn cook_complete(r_con_hold: Arc<Mutex<Connection>>) {
    let mut r_con = r_con_hold.lock().await;

    let mut cook_queue_ptr: u32 = r_con.get(COOK_QUEUE_PTR).unwrap();
    let key = cook_queue_key(cook_queue_ptr);
    let cooked_items: Vec<String> = r_con.smembers(&key).unwrap();
    let removed_items: HashSet<String> = r_con.smembers(REMOVED_ITEMS).unwrap();
    let mut pipe = &mut redis::pipe();

    print!("round {} cooked: ", cook_queue_ptr);
    for s in cooked_items.iter() {
        if (removed_items.contains(s)) {
            pipe = pipe.srem(REMOVED_ITEMS, s);
        } else {
            print!("{} ", s);
            let (table_no, item_no) = s.split_once("-").unwrap();
            pipe = pipe
                .hdel(format!("table:{}", table_no), item_no)
                .del(format!("item:{}-{}", table_no, item_no));
        }
    }
    println!();
    cook_queue_ptr = (cook_queue_ptr + 1) % COOK_QUEUE_LEN;
    let _ : () = pipe
        .del(&key)
        .set(COOK_QUEUE_PTR, cook_queue_ptr)
        .query(&mut (*r_con)).unwrap();
}