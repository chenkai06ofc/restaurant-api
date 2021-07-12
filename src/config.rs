use std::env;

// env variable keys
const ENV_REDIS_HOST: &str = "REDIS_HOST";
// how many seconds a minute contain, can be changed for TEST
const ENV_SECS_PER_MIN: &str = "SECS_PER_MIN";

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