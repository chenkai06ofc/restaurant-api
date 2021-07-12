use std::collections::HashMap;

pub fn parse_query_str(query_string: &str) -> HashMap<String, String> {
    let mut params: HashMap<String, String> = HashMap::new();
    let p = &mut params;
    query_string.split("&").for_each(|s| {
        let (key, value) = s.split_once("=").unwrap();
        p.insert(String::from(key), String::from(value));
    });
    return params;
}
