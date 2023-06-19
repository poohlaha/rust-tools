//! Redis 连接 - 单个
//! 集群需要 redis = { version = "0.23.0", features = [ "cluster"] }

use redis::{Client, Commands, Connection, ConnectionInfo, IntoConnectionInfo, RedisConnectionInfo, RedisResult};
use std::time::Duration;

pub struct Options {
    pub url: String,
    pub username: Option<String>,
    pub pwd: Option<String>,
    pub db: Option<i64>,
    pub timeout: Option<u64>
}

pub struct Redis {
    url: String,
    username: String,
    pwd: String,
    db: i64,
    timeout: Duration
}

impl Redis {

    /// 初始化函数
    pub(crate) fn new(opts: Options) -> Redis {
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
            url: String::from(opts.url),
            username: redis_username,
            pwd: redis_pwd,
            db: redis_db,
            timeout: redis_timeout
        };
    }

    /// 连接 Redis, 返回 Connection
    pub fn connect(&self) -> Option<Connection> {
        if self.url.is_empty() {
            println!("url is empty !");
            return None;
        }

        let mut redis_url = String::new();
        if !self.url.starts_with("redis") {
            redis_url = redis_url + "redis://";
        }

        redis_url += &self.url;
        // redis_url += &format!("?db={}?username={}&password={}", self.db, self.username, self.pwd);

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
    pub fn get_data<T: redis::FromRedisValue>(&self, client: &Redis, key: &str) -> RedisResult<T> {
        if let Some(mut client) = client.connect() {
            return client.get(key);
        } else {
            panic!("client is null .")
        }
    }
}