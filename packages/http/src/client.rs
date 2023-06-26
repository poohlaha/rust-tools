//! 使用 reqwest 发送 http 和 https 请求

use std::collections::HashMap;
use std::fmt::Debug;
use serde_json::Value;
use reqwest::{Client, Method, RequestBuilder};
use reqwest::header::{HeaderMap, HeaderName};
use crate::options::Options;
use crate::options::HttpResponse;

pub struct HttpClient;

impl HttpClient {

    /// 获取返回错误 HttpResponse
    fn get_error_response<T: Debug + ToString>(code: u16, error: &T) -> HttpResponse {
        return HttpResponse {
            status_code: code,
            headers: HashMap::new(),
            body: Value::default(),
            error: format!("send request error: {:?}", error)
        };
    }

    /// 获取请求头
    fn get_headers(headers: Option<Value>) -> Vec<(String, String)> {
        let mut new_headers: Vec<(String, String)> = Vec::new();

        let mut has_content_type: bool = false;
        if let Some(header) = headers {
            for(key, value) in header.as_object().unwrap() {
                if key.to_lowercase() == "content-type" {
                    has_content_type = true
                }

                let header_value = value.as_str().unwrap_or("");
                new_headers.push((key.clone(), String::from(header_value)));
            }
        }

        if !has_content_type {
            new_headers.push((String::from("content-type"), String::from("application/json")));
        }

        return new_headers;
    }

    /// 发送请求
    pub async fn send(options: Options) -> HttpResponse {
        println!("options: {:?}", options);

        if options.url.is_empty() {
            println!("url is empty");
            return HttpResponse {
                status_code: 500,
                headers: HashMap::new(),
                body: Value::default(),
                error: String::from("url is empty")
            };
        }

        // method
        let method: String = options.method.as_deref().unwrap_or("post").to_string();
        let request_method = if method.to_lowercase() == "get" {Method::GET} else {Method::POST}; // 发送请求 method

        let client = Client::new();
        let mut request: RequestBuilder = client.request(request_method, options.url);

        // headers
        let mut request_headers = HeaderMap::new();
        let headers = Self::get_headers(options.headers);
        for (name, value) in headers.iter() {
            request_headers.insert(&HeaderName::from_bytes(name.as_bytes()).unwrap(), value.as_str().parse().unwrap());
        }

        println!("headers: {:?}", request_headers);

        // body
        if let Some(data) = options.data {
            request = request.body(data.to_string());
        }

       match request.send().await {
            Ok(response) => {
                let status = response.status();
                let status_code = status.as_u16();
                if status.is_success() {
                    let response_headers = response.headers().clone();
                    let headers: HashMap<String, String> = response_headers.iter()
                        .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string()))
                        .collect();
                    let body = response.text().await.unwrap_or("".to_string());
                    return HttpResponse {
                        status_code: 200,
                        headers,
                        body: serde_json::from_slice(body.as_bytes()).unwrap(),
                        error: String::new(),
                    }
                } else {
                    return Self::get_error_response(status_code, &status_code);
                }
            },
            Err(error) => {
                println!("send request error: {}", error);
                return Self::get_error_response(500, &error);
            }
        };
    }
}