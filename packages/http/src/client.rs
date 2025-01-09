use crate::options::HttpResponse;
use crate::options::{HttpError, Options};
use crate::LOGGER_PREFIX;
use colored::*;
use reqwest::header::{HeaderMap, HeaderName};
use reqwest::{Client, Method, RequestBuilder, StatusCode};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Debug;
use std::time::Duration;

pub struct HttpClient;

const DEFAULT_TIMEOUT: u64 = 30;

impl HttpClient {
    /// return the error response
    fn get_error_response<T: Debug + ToString>(code: u16, error: &T) -> HttpResponse {
        return HttpResponse {
            status_code: code,
            headers: HashMap::new(),
            body: Value::default(),
            error: format!("send request error: {:?}", error),
        };
    }

    /// get headers
    fn get_headers(headers: Option<Value>, is_form_submit: bool, is_file_submit: bool) -> Vec<(String, String)> {
        let mut new_headers: Vec<(String, String)> = Vec::new();

        let mut has_content_type: bool = false;
        if let Some(header) = headers {
            for (key, value) in header.as_object().unwrap() {
                if key.to_lowercase() == "content-type" {
                    has_content_type = true
                }

                let header_value = value.as_str().unwrap_or("");

                new_headers.push((key.clone(), String::from(header_value)));
            }
        }

        if !has_content_type {
            if is_form_submit {
                new_headers.push((String::from("content-type"), String::from("application/x-www-form-urlencoded")));
            } else if is_file_submit {
                // if need will be error
                // new_headers.push((String::from("content-type"), String::from("multipart/form-data")));
            } else {
                new_headers.push((String::from("content-type"), String::from("application/json")));
            }
        }

        return new_headers;
    }

    /// send request
    pub async fn send(options: Options, is_form_submit: bool) -> Result<HttpResponse, HttpError> {
        // println!("{} options: {:#?}", LOGGER_PREFIX.cyan().bold(), options);

        if options.url.is_empty() {
            // println!("{} {}", LOGGER_PREFIX.cyan().bold(), "url is empty !".red().bold());
            return Err(HttpError::Empty("url is empty !".to_string()));
        }

        // method
        let method: String = options.method.as_deref().unwrap_or("post").to_string();
        let request_method = if method.to_lowercase() == "get" { Method::GET } else { Method::POST };

        // Client::new() | Client::builder()
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            // .danger_accept_invalid_hostnames(true)
            .build()
            .map_err(|err| HttpError::CreateClientError(Box::new(err)))?;

        let request: RequestBuilder = client.request(request_method, options.url);
        let mut request = request.timeout(Duration::from_secs(HttpClient::get_timeout(options.timeout)));

        // headers
        let mut request_headers = HeaderMap::new();
        let headers = Self::get_headers(options.headers, is_form_submit, false);
        for (name, value) in headers.iter() {
            request_headers.insert(&HeaderName::from_bytes(name.as_bytes()).unwrap(), value.as_str().parse().unwrap());
        }

        // println!("{} headers: {:#?}", LOGGER_PREFIX.cyan().bold(), request_headers);

        // body
        if let Some(data) = options.data {
            if is_form_submit {
                request = request.form(data.as_object().unwrap());
            } else {
                request = request.body(data.to_string());
            }
        }

        let response = request.headers(request_headers).send().await.map_err(|err| HttpError::ResponseError(Box::new(err)))?;
        let status = response.status();
        let response_headers = response.headers().clone();
        let body = response.text().await.unwrap_or("".to_string());
        Ok(HttpClient::get_response(status, response_headers, body))
    }

    /// send form-data request, use reqwest blocking
    pub fn send_form_data(options: Options) -> Result<HttpResponse, HttpError> {
        // println!("{} options: {:#?}", LOGGER_PREFIX.cyan().bold(), options);

        if options.url.is_empty() {
            // println!("{} {}", LOGGER_PREFIX.cyan().bold(), "url is empty !".red().bold());
            return Err(HttpError::Empty("url is empty !".to_string()));
        }

        // method
        let method: String = options.method.as_deref().unwrap_or("post").to_string();
        let request_method = if method.to_lowercase() == "get" { Method::GET } else { Method::POST };

        // Ignore `HTTPS` certificate
        let client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(true)
            // .danger_accept_invalid_hostnames(true)
            .build()
            .map_err(|err| HttpError::CreateClientError(Box::new(err)))?;

        let request = client.request(request_method, options.url);
        let mut request = request.timeout(Duration::from_secs(HttpClient::get_timeout(options.timeout)));

        // headers
        let mut request_headers = HeaderMap::new();
        let headers = Self::get_headers(options.headers, false, true);
        for (name, value) in headers.iter() {
            request_headers.insert(&HeaderName::from_bytes(name.as_bytes()).unwrap(), value.as_str().parse().unwrap());
        }

        // println!("{} headers: {:?}", LOGGER_PREFIX.cyan().bold(), request_headers);

        // form
        if let Some(form) = options.form {
            request = request.multipart(form);
        }

        let response = request.headers(request_headers).send().map_err(|err| HttpError::ResponseError(Box::new(err)))?;
        let status = response.status();
        let response_headers = response.headers().clone();
        let body = response.text().unwrap_or("".to_string());
        Ok(HttpClient::get_response(status, response_headers, body))
    }

    /// get http response
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

    /// get timeout
    fn get_timeout(timeout: Option<u64>) -> u64 {
        let mut send_timeout = DEFAULT_TIMEOUT;
        if !timeout.is_none() {
            send_timeout = timeout.unwrap();
        }

        return send_timeout;
    }
}
