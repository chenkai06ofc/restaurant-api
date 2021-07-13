use std::collections::HashMap;

pub fn parse_query_str(query_string: &str) -> HashMap<String, String> {
    query_string.split("&")
        .map(|s| s.split_once("=").unwrap())
        .map(|(s1, s2)| (String::from(s1), String::from(s2)))
        .collect()
}

#[cfg(test)]
mod test {
    use crate::util::parse_query_str;

    #[test]
    fn parse_query_str_test() {
        let map1 = parse_query_str("table_no=10");
        assert_eq!(map1.len(), 1);
        assert_eq!(map1.get("table_no").unwrap(), "10");

        let map2 = parse_query_str("table_no=5&item_no=6");
        assert_eq!(map2.len(), 2);
        assert_eq!(map2.get("table_no").unwrap(), "5");
        assert_eq!(map2.get("item_no").unwrap(), "6");

        let map3 = parse_query_str("table_no=5&item_no=6&content=apple");
        assert_eq!(map3.len(), 3);
        assert_eq!(map3.get("table_no").unwrap(), "5");
        assert_eq!(map3.get("item_no").unwrap(), "6");
        assert_eq!(map3.get("content").unwrap(), "apple");
    }
}