use serde::{Serialize, Deserialize};
use redis::{Commands, Connection};
use std::collections::HashMap;
use rand::Rng;

pub const MIN_TABLE_NO: u32 = 1;
pub const MAX_TABLE_NO: u32 = 100;

static mut cur_item_no: u64 = 1;
const COOK_QUEUE_LEN: u32 = 20;

// redis keys
pub const COOK_QUEUE_PTR: &str = "cook_queue_ptr";

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

unsafe fn next_item_no() -> u64 {
    let item_no = cur_item_no;
    cur_item_no+=1;
    item_no
}

fn table_key(table_no: u32) -> String {
    format!("table:{}", table_no)
}

fn item_key(table_no: u32, item_no: u64) -> String {
    format!("item:{}-{}", table_no, item_no)
}

fn cook_queue_key(cook_queue_ptr: u32) -> String {
    format!("cook_queue:{}", cook_queue_ptr)
}

pub fn get_item(con: &mut Connection, table_no: u32, item_no: u64) -> Item {
    let map : HashMap<String, String> =
        con.hgetall(item_key(table_no, item_no)).unwrap();

    Item::new(
        table_no,
        item_no,
        String::from(map.get("content").unwrap()),
        map.get("prepare_time_min").unwrap().parse().unwrap()
    )
}

pub unsafe fn add_item(con: &mut Connection, table_no: u32, content: &String) {
    let item_no = next_item_no();

    let table_key = table_key(table_no);
    let item_key = item_key(table_no, item_no);
    let prepare_time_min = rand::thread_rng().gen_range(5..15);
    println!("  add {}, time: {}", &item_key, prepare_time_min);
    let time_str = prepare_time_min.to_string();

    // update redis Set cook_queue:{min}
    let mut cook_queue_ptr: u32 = con.get(COOK_QUEUE_PTR).unwrap();
    cook_queue_ptr = (cook_queue_ptr + prepare_time_min) % COOK_QUEUE_LEN;

    let _ : () = redis::pipe()
        .hset(&table_key, &item_no.to_string(), content)
        .hset_multiple(&item_key, &[("content", content), ("prepare_time_min", &time_str) ])
        .sadd(cook_queue_key(cook_queue_ptr), format!("{}-{}",table_no, item_no))
        .query(con).unwrap();
}

pub fn remove_item(con: &mut Connection, table_no: u32, item_no: u64) {
    let table_key = table_key(table_no);
    let item_key = item_key(table_no, item_no);
    let _ : () = redis::pipe().hdel(&table_key, item_no).del(&item_key).query(con).unwrap();
}

pub fn cook_complete(con: &mut Connection) {
    let mut cook_queue_ptr: u32 = con.get(COOK_QUEUE_PTR).unwrap();
    let key = cook_queue_key(cook_queue_ptr);
    let vec: Vec<String> = con.smembers(&key).unwrap();
    println!("round {} cooked: {}", cook_queue_ptr, vec.join(" "));
    let mut pipe = redis::pipe();
    let mut p = &mut pipe;
    for s in vec.iter() {
        let (table_no, item_no) = s.split_once("-").unwrap();
        p = p.hdel(format!("table:{}", table_no), item_no).del(format!("item:{}-{}", table_no, item_no));
    }
    cook_queue_ptr = (cook_queue_ptr + 1) % COOK_QUEUE_LEN;
    let _ : () = p.del(&key).set(COOK_QUEUE_PTR, cook_queue_ptr).query(con).unwrap();
}