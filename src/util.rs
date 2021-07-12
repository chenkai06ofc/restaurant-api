use std::collections::HashMap;

pub fn parse_query_str(query_string: &str) -> HashMap<String, String> {
    query_string.split("&")
        .map(|s| s.split_once("=").unwrap())
        .map(|(s1, s2)| (String::from(s1), String::from(s2)))
        .collect()
}
