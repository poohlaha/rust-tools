//! 文件下载

use std::ffi::OsStr;
use std::fs::{File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use colored::*;
use reqwest::{Client};
use crate::LOGGER_PREFIX;

pub struct Download;

pub struct DownloadOptions {
    pub url: String, // url
    pub file_name: Option<String>, // 另存为的文件名, 如果为空, 则取 url 中的文件名
    pub timeout: Option<u64>, // 超时时间
    pub output_dir: Option<String>, // 输出目录
}

const TIMEOUT: u64 = 30;
impl Download {

    /// 文件下载
    pub async fn download(options: DownloadOptions) -> bool {
        if options.url.is_empty() {
            println!("{} download url is empty !", LOGGER_PREFIX.cyan().bold());
            return false;
        }

        let file_name = options.file_name.clone();
        let download_file_name;
        if file_name.is_none() {
           let file_path = Path::new(&options.url);
            download_file_name = file_path.file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
        } else {
            download_file_name = file_name.unwrap();
        }

        if download_file_name.is_empty() {
            println!("{} download file name is empty, please check `url` or `file_name` !", LOGGER_PREFIX.cyan().bold());
            return false;
        }

        let timeout = options.timeout;
        let download_timeout = if timeout.is_none() { TIMEOUT } else {timeout.unwrap()};

        let client = match Client::builder().timeout(Duration::new(download_timeout, 0)).build() {
            Ok(client) => Some(client),
            Err(err) => {
                println!("{} create client error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if client.is_none() {
            return false;
        }

        let client = client.unwrap();
        let response = match client.get(options.url.clone()).send().await {
            Ok(response) => Some(response),
            Err(err) => {
                println!("{} get response error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if response.is_none() {
            return false;
        }

        let response = response.unwrap();
        if response.status().is_success() {
            let content = match response.bytes().await {
                Ok(content) => Some(content),
                Err(err) => {
                    println!("{} get response error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                    None
                }
            };

            if content.is_none() {
                return false;
            }

            let mut output_file_path = PathBuf::new();
            if options.output_dir.is_some() {
                let output_dir = options.output_dir.clone().unwrap();
                output_file_path.push(&output_dir);
            }

            output_file_path.push(&download_file_name);
            println!("{} download file path: {}", LOGGER_PREFIX.cyan().bold(), output_file_path.as_path().to_string_lossy().to_string());
            let dest_file = match File::create(output_file_path) {
                Ok(file) => Some(file),
                Err(err) => {
                    println!("{} create file error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                    None
                }
            };

            if dest_file.is_none() {
                return false;
            }

            let mut dest_file = dest_file.unwrap();
            let content = content.unwrap();
            let success = match io::copy(&mut content.as_ref(), &mut dest_file) {
                Ok(_) => true,
                Err(err) => {
                    println!("{} download file error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                    false
                }
            };

            if !success {
                return false;
            }

            println!("{} download file successfully !", LOGGER_PREFIX.cyan().bold());
            return true;
        } else {
            println!("{} download file failed with status code: {}", LOGGER_PREFIX.cyan().bold(), response.status());
        }

        return false
    }
}