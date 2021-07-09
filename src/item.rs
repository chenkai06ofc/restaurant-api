use serde::{Serialize, Deserialize};
use redis::{Commands, Connection};
use std::collections::HashMap;

pub const MIN_TABLE_NO: u32 = 1;
pub const MAX_TABLE_NO: u32 = 100;

#[derive(Serialize, Deserialize)]
pub struct Item {
    table_no: u32,
    item_no: u32,
    content: String,
    //generate_time:
    prepare_time_min: u32,
}

impl Item {
    pub fn new(table_no: u32, item_no: u32, content: String, prepare_time_min: u32) -> Item {
        Item {
            table_no,
            item_no,
            content,
            prepare_time_min
        }
    }
}


pub fn get_item(con: &mut Connection, table_no: u32, item_no: u32) -> Item {
    let map : HashMap<String, String> =
        con.hgetall(format!("item:{}-{}", table_no, item_no)).unwrap();

    Item::new(
        table_no,
        item_no,
        String::from(map.get("content").unwrap()),
        map.get("prepare_time_min").unwrap().parse().unwrap()
    )
}