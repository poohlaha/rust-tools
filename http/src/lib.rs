//! 对外发送请求, 并转成 `wasm`

pub mod client;
pub mod options;
use client::HttpClient;
use options::HttpResponse;
use options::Options;

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::{to_value, from_value};

/**
  opts: {
   urls: '',
   method: '',
   data: {},
   header: {}
 }
*/
#[wasm_bindgen]
pub async fn send(opts: JsValue) -> Result<JsValue, JsValue> {
    let options: Options = from_value(opts).unwrap();
    let response: HttpResponse = HttpClient::send(options).await;
    println!("response: {:#?}", response);

    // 将结果转换为适合与 WebAssembly 交互的类型
    Ok(to_value(&response)?)
}

/// 发送请求
pub async fn client_send(opts: Options) -> HttpResponse {
    let response: HttpResponse = HttpClient::send(opts).await;
    println!("response: {:#?}", response);
    return response;
}
