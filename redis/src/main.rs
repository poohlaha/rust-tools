//! 连接 redis

mod client;
use client::Redis;
use client::Options;

fn main() {
   let options = Options {
      host: "47.100.222.31".to_string(),
      port: None,
      username: None,
      pwd: Some("%1ZwpH3kzxHrq3KLh".to_string()),
      db: Some(0),
      timeout: None,
   };
   let client = Redis::new(options);
   let mut connection = client.connect();
   client.set_data(&mut connection, "hello", "test-23456");
   let result: Option<String> = client.get_data(&mut connection, "hello");
   print!("result: {:#?}", result.unwrap());
}

