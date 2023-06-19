//! 连接 redis

mod client;

use redis::RedisResult;
use client::Redis;
use client::Options;

fn main() {
   let options = Options {
      url: "47.113.141.61:6379".to_string(),
      username: None,
      pwd: Some("%ZwpH&mkz1xHrq5KLh".to_string()),
      db: Some(0),
      timeout: None,
   };
   let client = Redis::new(options);
   let result: RedisResult<String> = client.get_data(&client, "test");
   print!("result: {:#?}", result);
}
