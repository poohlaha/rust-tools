//! 输入和输出数据结构体

use serde_json::Value;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct Options {
    pub url: String, // url
    pub method: Option<String>, // method: post、get
    pub data: Option<Value>, // data
    pub headers: Option<Value> // headers
}

/// 输出数据结构体
#[derive(Serialize, Deserialize, Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Value,
    pub error: String
}
