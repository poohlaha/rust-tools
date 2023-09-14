//! 对外发送请求, 并转成 `wasm`

pub mod client;
pub mod options;
pub mod download;

use client::HttpClient;
use colored::*;
use options::HttpResponse;
use options::Options;
use crate::download::{Download, DownloadOptions};

const LOGGER_PREFIX: &str = "[Http]: ";

/**
  opts: {
   urls: '',
   method: '',
   data: {},
   header: {}
 }
*/
/// 发送请求
pub async fn client_send(opts: Options, is_form_submit: bool) -> HttpResponse {
    let response: HttpResponse = HttpClient::send(opts, is_form_submit).await;
    println!("{} response: {:#?}", LOGGER_PREFIX.cyan().bold(), response);
    return response;
}

/// 发送请求
pub fn client_send_form_data(opts: Options) -> HttpResponse {
    let response: HttpResponse = HttpClient::send_form_data(opts);
    println!("{} response: {:#?}", LOGGER_PREFIX.cyan().bold(), response);
    return response;
}

/// 文件下载
pub async fn download(options: DownloadOptions) -> bool {
    return Download::download(options).await;
}
