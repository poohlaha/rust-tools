//! 使用 reqwest 发送 http 和 https 请求

use std::collections::HashMap;
use std::fmt::Debug;
use serde_json::Value;
use reqwest::{Client, Method, RequestBuilder, StatusCode};
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
    fn get_headers(headers: Option<Value>, is_form_submit: bool, is_file_submit: bool) -> Vec<(String, String)> {
        let mut new_headers: Vec<(String, String)> = Vec::new();

        let mut has_content_type: bool = false;
        if let Some(header) = headers {
            for (key, value) in header.as_object().unwrap() {
                if key.to_lowercase() == "content-type" {
                    has_content_type = true
                }

                let mut header_value = value.as_str().unwrap_or("");
                // 表单提交
                if is_form_submit {
                    if header_value.to_lowercase() != "application/x-www-form-urlencoded" {
                        header_value = "application/x-www-form-urlencoded";
                    }
                }

                // 上传文件
                if is_file_submit {
                    if header_value.to_lowercase() != "multipart/form-data" {
                        header_value = "multipart/form-data";
                    }
                }

                new_headers.push((key.clone(), String::from(header_value)));
            }
        }

        if !has_content_type {
            if is_form_submit {
                new_headers.push((String::from("content-type"), String::from("application/x-www-form-urlencoded")));
            } else if is_file_submit {
                new_headers.push((String::from("content-type"), String::from("multipart/form-data")));
            } else {
                new_headers.push((String::from("content-type"), String::from("application/json")));
            }
        }

        return new_headers;
    }

    /// 发送请求
    pub async fn send(options: Options, is_form_submit: bool) -> HttpResponse {
        println!("options: {:#?}", options);

        if options.url.is_empty() {
            println!("url is empty");
            return HttpResponse {
                status_code: 500,
                headers: HashMap::new(),
                body: Value::default(),
                error: String::from("url is empty"),
            };
        }

        // method
        let method: String = options.method.as_deref().unwrap_or("post").to_string();
        let request_method = if method.to_lowercase() == "get" { Method::GET } else { Method::POST }; // 发送请求 method

        let client = Client::new();
        let mut request: RequestBuilder = client.request(request_method, options.url);

        // headers
        let mut request_headers = HeaderMap::new();
        let headers = Self::get_headers(options.headers, is_form_submit, false);
        for (name, value) in headers.iter() {
            request_headers.insert(&HeaderName::from_bytes(name.as_bytes()).unwrap(), value.as_str().parse().unwrap());
        }

        println!("headers: {:?}", request_headers);

        // body
        if let Some(data) = options.data {
            if is_form_submit {
                request = request.form(data.as_object().unwrap());
            } else {
                request = request.body(data.to_string());
            }
        }

        match request.send().await {
            Ok(response) => {
                let status = response.status();
                let response_headers = response.headers().clone();
                let body = response.text().await.unwrap_or("".to_string());
                return HttpClient::get_response(status, response_headers, body);
            }
            Err(error) => {
                println!("send request error: {}", error);
                return Self::get_error_response(500, &error);
            }
        };
    }

    /// 通过 multipart/form-data 提交, 使用 blocking
    pub fn send_form_data(options: Options) -> HttpResponse {
        println!("options: {:#?}", options);

        if options.url.is_empty() {
            println!("url is empty");
            return HttpResponse {
                status_code: 500,
                headers: HashMap::new(),
                body: Value::default(),
                error: String::from("url is empty"),
            };
        }

        // method
        let method: String = options.method.as_deref().unwrap_or("post").to_string();
        let request_method = if method.to_lowercase() == "get" { Method::GET } else { Method::POST }; // 发送请求 method

        let client = reqwest::blocking::Client::new();
        let mut request = client.request(request_method, options.url);

        // headers
        let mut request_headers = HeaderMap::new();
        let headers = Self::get_headers(options.headers, false, true);
        for (name, value) in headers.iter() {
            request_headers.insert(&HeaderName::from_bytes(name.as_bytes()).unwrap(), value.as_str().parse().unwrap());
        }

        println!("headers: {:?}", request_headers);

        // form
        if let Some(form) = options.form {
            request = request.multipart(form);
        }

        match request.send() {
            Ok(response) => {
                let status = response.status();
                let response_headers = response.headers().clone();
                let body = response.text().unwrap_or("".to_string());
                return HttpClient::get_response(status, response_headers, body);
            }
            Err(error) => {
                println!("send request error: {}", error);
                return Self::get_error_response(500, &error);
            }
        };
    }

    /// 获取 response
    fn get_response(status: StatusCode, response_headers: HeaderMap, body: String) -> HttpResponse {
        let status_code = status.as_u16();
        if status.is_success() {
            let headers: HashMap<String, String> = response_headers.iter().map(|(name, value)| (name.to_string(), value.to_str().unwrap_or("").to_string())).collect();
            return HttpResponse {
                status_code: 200,
                headers,
                body: serde_json::from_slice(body.as_bytes()).unwrap(),
                error: String::new(),
            };
        } else {
            return Self::get_error_response(status_code, &status_code);
        }
    }
}
