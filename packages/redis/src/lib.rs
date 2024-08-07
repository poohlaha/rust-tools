//! 连接 redis

mod client;
use client::Options;
use client::Redis;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis() {
        let options = Options {
            host: "47.230.232.31".to_string(),
            port: None,
            username: None,
            pwd: Some("%1ZwpH3kzxHdrq3KLh".to_string()),
            db: Some(0),
            timeout: None,
        };
        let client = Redis::new(options);
        let mut connection = client.connect();
        client.set_data(&mut connection, "hello", "test-23456");
        let result: Option<String> = client.get_data(&mut connection, "hello");
        println!("result: {}", result.unwrap());
    }
}
