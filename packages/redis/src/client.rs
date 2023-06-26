//! Redis 连接 - 单个
//! 集群需要 redis = { version = "0.23.0", features = [ "cluster"] }

use redis::{Client, Commands, Connection, ConnectionInfo, IntoConnectionInfo, RedisConnectionInfo};
use std::time::Duration;

pub struct Options {
    pub host: String,
    pub port: Option<u32>,
    pub username: Option<String>,
    pub pwd: Option<String>,
    pub db: Option<i64>,
    pub timeout: Option<u64>
}

pub struct Redis {
    host: String,
    port: u32,
    username: String,
    pwd: String,
    db: i64,
    timeout: Duration
}

impl Redis {

    /// 初始化函数
    pub(crate) fn new(opts: Options) -> Redis {
        // port
        let mut redis_port = 6379;
        if let Some(port) = opts.port {
            redis_port = port;
        }


        // username
        let mut redis_username = String::new();
        if let Some(username) = opts.username {
            redis_username = username;
        }

        // pwd
        let mut redis_pwd = String::new();
        if let Some(pwd) = opts.pwd {
            redis_pwd = pwd;
        }

        // timeout
        let mut redis_timeout: Duration = Duration::from_millis(10000);
        if let Some(time) = opts.timeout {
            redis_timeout = Duration::from_millis(time);
        }

        // db, 默认为 0
        let mut redis_db: i64 = 0;
        if let Some(db) = opts.db {
            redis_db = db;
        }

        return Redis {
            host: String::from(opts.host),
            port: redis_port,
            username: redis_username,
            pwd: redis_pwd,
            db: redis_db,
            timeout: redis_timeout
        };
    }

    /// 连接 Redis, 返回 Connection
    pub fn connect(&self) -> Option<Connection> {
        if self.host.is_empty() {
            println!("host is empty !");
            return None;
        }

        let mut redis_url = String::new();
        redis_url = redis_url + "redis://";
        redis_url += &self.host;
        redis_url += ":";
        redis_url += &self.port.to_string();

        // 通过 connectionInfo 方式连接
        let mut connection_info: ConnectionInfo = match redis_url.clone().into_connection_info() {
            Ok(info) => info,
            Err(err) => {
                println!("connect to redis failed, error: {:?}", err);
                return None;
            }
        };

        let username = &self.username;
        let pwd = &self.pwd;
        let connection_redis = RedisConnectionInfo {
            db: self.db,
            username: Some(username.to_string()),
            password: Some(pwd.to_string()),
        };

        connection_info.redis = connection_redis;

        let client: Option<Client> = match Client::open(connection_info) {
            Ok(client) => Some(client),
            Err(err) => {
                println!("connect to redis failed, error: {:?}", err);
                return None;
            }
        };

        if let Some(client) = client {
            match client.get_connection_with_timeout(self.timeout) {
                Ok(connect) => Some(connect),
                Err(err) => {
                    println!("connect to redis failed, error: {:?}", err);
                    return None;
                }
            }
        } else {
            return None;
        }
    }

    /// 根据 key 获取数据
    pub fn get_data<T: redis::FromRedisValue>(&self, connect: &mut Option<Connection>, key: &str) -> Option<T> {
        if key.is_empty() {
            println!("key is null .");
            return None;
        }

        match connect.as_mut() {
            None => panic!("client is null ."),
            Some(connection) => {
               return match connection.get(key)  {
                    Ok(value) => Some(value),
                    Err(error) => {
                        println!("get key: {} error: {:?}", key, error);
                        return None;
                    }
                };
            }
        }
    }

    /// 设置值
    pub fn set_data(&self, connect: &mut Option<Connection>, key: &str, value: &str) -> bool {
        if key.is_empty() {
            println!("key is null .");
            return false;
        }

        if value.is_empty() {
            println!("value is null .");
            return false;
        }

        return match connect.as_mut() {
            None => {
                println!("client is null .");
                return false;
            },
            Some(connection) => {
                match connection.set::<&str, &str, String>(key, value) {
                    Ok(_) => true,
                    Err(err) => {
                        println!("set key: {} error: {:?}", key, err);
                        return false
                    }
                }
            }
        }
    }
}