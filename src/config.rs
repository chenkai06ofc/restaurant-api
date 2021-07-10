use std::env;

// env variable keys
const ENV_REDIS_HOST: &str = "REDIS_HOST";

pub fn redis_url() -> String {
    let redis_host = match env::var(ENV_REDIS_HOST) {
        Ok(v) => v,
        Err(_) => String::from("localhost")
    };
    format!("redis://{}/", redis_host)
}