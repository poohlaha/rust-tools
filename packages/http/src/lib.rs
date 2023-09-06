//! 对外发送请求

pub mod client;
pub mod options;
use client::HttpClient;
use options::HttpResponse;
use options::Options;

/// 发送请求
pub async fn client_send(opts: Options, is_form_submit: bool) -> HttpResponse {
    let response: HttpResponse = HttpClient::send(opts, is_form_submit).await;
    println!("response: {:#?}", response);
    return response;
}


/// 发送请求
pub fn client_send_form_data(opts: Options) -> HttpResponse {
    let response: HttpResponse = HttpClient::send_form_data(opts);
    println!("response: {:#?}", response);
    return response;
}

