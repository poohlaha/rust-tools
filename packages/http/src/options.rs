use crate::HttpFormData;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

#[derive(Debug)]
pub struct Options {
    pub url: String,                // url
    pub method: Option<String>,     // method: post、get
    pub data: Option<Value>,        // data
    pub form: Option<HttpFormData>, // form
    pub headers: Option<Value>,     // headers
    pub timeout: Option<u64>,       // timeout
}

#[derive(Serialize, Deserialize, Debug)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: Value,
    pub error: String,
}

#[derive(thiserror::Error, Debug)]
pub enum HttpError {
    #[error("`{0}`")]
    Empty(String),

    #[error("{0}")]
    Error(Box<dyn Error>),

    #[error("create http client error: {0}")]
    CreateClientError(Box<dyn Error>),

    #[error("send request error: {0}")]
    SendError(Box<dyn Error>),

    #[error("get response error: {0}")]
    ResponseError(Box<dyn Error>),
}
