//! 前端上传日志到后台，然后存储到文件中

use serde_json::{from_str, Value};
use std::fs;
use std::io::{Result, Write};
use std::net::TcpListener;
use std::thread;
use tungstenite::accept;

// 设置 HOST
const HOST: &str = "127.0.0.1:7878";
const LOGGER_PREFIX: &str = "[Rust Web Server]: ";
const LOG_FILE_PATH: &str = "/Users/smile/tools/logs/client_error.log";

/// 启动服务器
fn run() -> TcpListener {
    let server: TcpListener = match TcpListener::bind(HOST) {
        Ok(server) => {
            println!("{} WebSocket server listening on ws://{}", LOGGER_PREFIX, HOST);
            server
        }
        Err(err) => panic!("{} WebSocket server start error: {:?}", LOGGER_PREFIX, err),
    };

    return server;
}

// 写入日志文件
fn write_to_log_file(data: Value) -> Result<()> {
    // 判断日志文件是否存在, 不存在则创建
    let stack = data.get("stack").and_then(Value::as_str).unwrap().to_string();
    let error = data.get("error").and_then(Value::as_str).unwrap().to_string();

    let mut error_data = String::from("error: \n");
    error_data += &"stack: \n";
    error_data += &stack;
    error_data += &"error \n";
    error_data += &error;
    println!("error_data {}", error_data);

    let mut file = fs::OpenOptions::new().create(true).append(true).open(LOG_FILE_PATH)?;
    writeln!(file, "{}", error_data)?;
    Ok(())
}

fn main() {
    let server: TcpListener = run();
    for stream in server.incoming() {
        thread::spawn(move || {
            let mut websocket = accept(stream.expect("Failed to accept connection")).unwrap();
            println!("{} WebSocket client connected !", LOGGER_PREFIX);

            let msg = websocket.read_message().expect("Failed to read message !");
            if msg.is_binary() || msg.is_text() {
                let received_data = msg.to_text().unwrap();
                println!("{} Received data: {}", LOGGER_PREFIX, received_data);

                // 将字符串解析为 JSON 值
                let data: Value = from_str::<Value>(&received_data).unwrap();

                // 在这里可以对接收到的消息进行处理
                match write_to_log_file(data) {
                    Ok(_) => {
                        println!("{} write to log success !", LOGGER_PREFIX);
                        // 发送响应给客户端
                        websocket.write_message("success".into()).expect("Failed to send response !");
                    }
                    Err(_) => {
                        websocket.write_message("failed".into()).expect("Failed to send response !");
                    }
                };
            }
        });
    }
}
