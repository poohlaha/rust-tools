//! 文件下载

use std::cmp::min;
use std::ffi::OsStr;
use std::fmt::Write;
use std::fs::{File};
use std::{fs, io};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use reqwest::{Client, Response};
use reqwest::header::CONTENT_LENGTH;
use tokio::fs::File as TokioFile;
use tokio::io::AsyncWriteExt;
use crate::LOGGER_PREFIX;

pub struct Download;

pub struct DownloadOptions {
    pub url: String, // url
    pub file_name: Option<String>, // 另存为的文件名, 如果为空, 则取 url 中的文件名
    pub timeout: Option<u64>, // 超时时间, 0 表示不设置超时时间
    pub output_dir: Option<String>, // 输出目录
    pub overwrite: Option<bool>, // 当文件存在时, 是否覆盖
}

#[derive(Default, Debug)]
pub struct DownloadResult {
    pub dir: String,
    pub success: bool,
    pub file_name: String,
    pub url: String,
}

impl DownloadResult {
    pub fn is_empty(args: &DownloadResult) -> bool {
        return args.dir.is_empty() && args.file_name.is_empty() && args.url.is_empty();
    }
}

const TIMEOUT: u64 = 30;
impl Download {

    fn get_file_name(options: &DownloadOptions) -> String {
        let file_name = options.file_name.clone();
        let download_file_name;
        if file_name.is_none() {
            let file_path = Path::new(&options.url);
            download_file_name = file_path.file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
        } else {
            download_file_name = file_name.unwrap();
        }

        return download_file_name;
    }

    fn get_timeout(options: &DownloadOptions) -> u64 {
        let timeout = options.timeout;
        let download_timeout = if timeout.is_none() { TIMEOUT } else {timeout.unwrap()};
        return download_timeout;
    }

    async fn get_response(options: &DownloadOptions) -> (Option<Response>, String) {
        if options.url.is_empty() {
            println!("{} download url is empty !", LOGGER_PREFIX.cyan().bold());
            return (None, String::new());
        }

        let download_file_name= Download::get_file_name(&options);
        if download_file_name.is_empty() {
            println!("{} download file name is empty, please check `url` or `file_name` !", LOGGER_PREFIX.cyan().bold());
            return (None, String::new());
        }

        let timeout = Download::get_timeout(&options);
        let client;
        if timeout <= 0 {
            client = match Client::builder().build() {
                Ok(client) => Some(client),
                Err(err) => {
                    println!("{} create client error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                    None
                }
            };
        } else {
            client = match Client::builder().timeout(Duration::new(timeout, 0)).build() {
                Ok(client) => Some(client),
                Err(err) => {
                    println!("{} create client error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                    None
                }
            };
        }

        if client.is_none() {
            return (None, String::new());
        }

        let client = client.unwrap();
        let response = match client.get(options.url.clone()).send().await {
            Ok(response) => Some(response),
            Err(err) => {
                println!("{} get response error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        return (response, download_file_name);
    }

    fn get_output_file(options: &DownloadOptions, download_file_name: &str) -> PathBuf {
        let mut output_file_path = PathBuf::new();
        if options.output_dir.is_some() {
            let output_dir = options.output_dir.clone().unwrap();
            output_file_path.push(&output_dir);
        }

        output_file_path.push(&download_file_name);
        return output_file_path;
    }

    /// 文件下载
    #[allow(dead_code)]
    async fn _download(options: DownloadOptions) -> DownloadResult {
        let mut result = DownloadResult::default();
        result.url = options.url.clone();
        result.dir = options.output_dir.clone().unwrap_or(String::new());

        let (response, download_file_name) = Download::get_response(&options).await;
        if response.is_none() || download_file_name.is_empty() {
            return result;
        }

        result.file_name = download_file_name.clone();
        let response = response.unwrap();

        if response.status().is_success() {
            let content = match response.bytes().await {
                Ok(content) => Some(content),
                Err(err) => {
                    println!("{} download file {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), &download_file_name, err);
                    None
                }
            };

            if content.is_none() {
                return result;
            }

            let content = content.unwrap();
            let output_file_path = Download::get_output_file(&options, &download_file_name);
            println!("{} download file path: {}", LOGGER_PREFIX.cyan().bold(), output_file_path.as_path().to_string_lossy().to_string());

            let overwrite = if options.overwrite.is_none() { true } else { options.overwrite.unwrap() };
            let mut has_need_download = true;

            if output_file_path.exists() {
                let size = fs::metadata(&output_file_path).unwrap().len();
                if size == content.len() as u64 {
                    has_need_download = overwrite
                }
            }

            if !has_need_download {
                println!("{} file has exists, skip !", LOGGER_PREFIX.cyan().bold());
                return result;
            }

            let dest_file = match File::create(output_file_path) {
                Ok(file) => Some(file),
                Err(err) => {
                    println!("{} download file {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), &download_file_name, err);
                    None
                }
            };

            if dest_file.is_none() {
                return result;
            }

            let mut dest_file = dest_file.unwrap();
            let success = match io::copy(&mut content.as_ref(), &mut dest_file) {
                Ok(_) => true,
                Err(err) => {
                    println!("{} download file {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), &download_file_name, err);
                    false
                }
            };

            if !success {
                return result;
            }

            result.success = true;
            println!("{} download file {} successfully !", LOGGER_PREFIX.cyan().bold(), &download_file_name.cyan().bold());
            return result;
        } else {
            println!("{} download file {} failed with status code: {}", LOGGER_PREFIX.cyan().bold(), &download_file_name.cyan().bold(), response.status());
        }

        return result
    }

    /// 文件下载, 包含进度条
    pub async fn download(options: DownloadOptions, progress: Option<&MultiProgress>) -> DownloadResult {
        let mut result = DownloadResult::default();
        result.url = options.url.clone();
        result.dir = options.output_dir.clone().unwrap_or(String::new());

        let (response, download_file_name) = Download::get_response(&options).await;
        // println!("{} download_file_name: {}", LOGGER_PREFIX.cyan().bold(), &download_file_name.cyan().bold());
        if response.is_none() || download_file_name.is_empty() {
            return result;
        }

        result.file_name = download_file_name.clone();
        let mut response = response.unwrap();
        if !response.status().is_success() {
            println!("{} download file {} failed with status code: {}", LOGGER_PREFIX.cyan().bold(), &download_file_name.cyan().bold(), response.status());
            return result;
        }

        // 获取响应头中的文件大小
        let content_length = response.headers().get(CONTENT_LENGTH).and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);

        if progress.is_none() {
            println!("{} file: {} content length: {}", LOGGER_PREFIX.cyan().bold(), &download_file_name, content_length);
        }

        if content_length == 0 {
            println!("{} url: {} the file on the server is empty !", LOGGER_PREFIX.cyan().bold(), &options.url);
            return result;
        }

        let output_file_path = Download::get_output_file(&options, &download_file_name);
        if progress.is_none() {
            println!("{} download file path: {}", LOGGER_PREFIX.cyan().bold(), output_file_path.as_path().to_string_lossy().to_string());
        }

        let overwrite = if options.overwrite.is_none() { true } else { options.overwrite.unwrap() };
        let mut has_need_download = true;

        // 判断当前文件是否存在, 并且是否已下载完成
        if output_file_path.exists() {
            let size = fs::metadata(&output_file_path).unwrap().len();
            if size == content_length {
                has_need_download = overwrite
            }
        }

        if !has_need_download {
            if progress.is_none() {
                println!("{} file has exists, skip !", LOGGER_PREFIX.cyan().bold());
            }
            result.success = true;
            return result;
        }

        let file = match TokioFile::create(&output_file_path).await {
            Ok(file) => Some(file),
            Err(err) => {
                println!("{} create file {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), output_file_path.as_path().to_string_lossy().to_string().red().bold(), err);
                None
            }
        };

        if file.is_none() {
            return result;
        }

        let mut file = file.unwrap();
        let mut downloaded_size = 0u64;
        let mut time = Instant::now();
        let mut download_speed = 0.0; // 下载速度

        let pb;
        if progress.is_none() {
            pb = ProgressBar::new(content_length);
        } else {
           let progress = progress.unwrap();
            pb = progress.add(ProgressBar::new(content_length));
        }

        let download_file_name_clone = download_file_name.clone();
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({msg}) ({eta})")
            .unwrap()
            .with_key("msg", move |_state: &ProgressState, w: &mut dyn Write| {
                write!(w, "{}", download_file_name_clone).unwrap()
            })
            .with_key("eta", |state: &ProgressState, w: &mut dyn Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("#>-"));


        // download
        while let Some(chunk) = response.chunk().await.unwrap() {
            let chunk_size = chunk.len() as u64;
            downloaded_size += chunk_size;

            let flag = match file.write_all(&chunk).await {
                Ok(_) => true,
                Err(err) => {
                    println!("{} write to file {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), &download_file_name.red().bold(), err);
                    false
                }
            };

            if !flag {
                return result;
            }

            // 计算下载速度
            let elapsed_time = time.elapsed().as_secs_f64();
            if elapsed_time >= 1.0 {
                download_speed = (downloaded_size as f64 / elapsed_time) / 1_000.0; // kbps
                time = Instant::now();
            }

            // 更新进度条
            pb.set_position(min(downloaded_size, content_length));
        }

        pb.finish_with_message(" ");

        let success = match file.sync_all().await {
            Ok(_) => true,
            Err(err) => {
                println!("{} download file {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), &download_file_name, err);
                false
            }
        };

        if !success {
            println!("{} download file {} failed !", LOGGER_PREFIX.cyan().bold(), &download_file_name.cyan().bold());
            return result;
        }

        if progress.is_none() {
            println!("{} download file {} successfully !", LOGGER_PREFIX.cyan().bold(), &download_file_name.cyan().bold());
        }

        result.success = true;
        return result;
    }
}