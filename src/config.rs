use std::env;
use mysql::Opts;

// env variable keys
const ENV_REDIS_HOST: &str = "REDIS_HOST";
// how many seconds a minute contain, can be changed for TEST
const ENV_SECS_PER_MIN: &str = "SECS_PER_MIN";
const ENV_MYSQL_HOST: &str = "MYSQL_HOST";
const ENV_MYSQL_PASSWORD: &str = "MYSQL_PASSWORD";
const ENV_MYSQL_TABLE_NAME: &str = "MYSQL_TABLE_NAME";

pub fn redis_url() -> String {
    let redis_host = match env::var(ENV_REDIS_HOST) {
        Ok(v) => v,
        Err(_) => String::from("localhost")
    };
    format!("redis://{}/", redis_host)
}

pub fn seconds_per_min() -> u64 {
    match env::var(ENV_SECS_PER_MIN) {
        Ok(s) => s.parse::<u64>().unwrap(),
        Err(_) => 60
    }
}

pub fn mysql_opts() -> Opts {
    let host = match env::var(ENV_MYSQL_HOST) {
        Ok(s) => s,
        Err(_) => String::from("localhost")
    };

    let password = match env::var(ENV_MYSQL_PASSWORD) {
        Ok(s) => s,
        Err(_) => String::from("")
    };

    let table_name = match env::var(ENV_MYSQL_TABLE_NAME) {
        Ok(s) => s,
        Err(_) => String::from("restaurant_api")
    };

    let url = format!("mysql://root:{}@{}:3306/{}", password, host, table_name);
    Opts::from_url(&url[..]).unwrap()
}