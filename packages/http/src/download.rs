use crate::options::HttpError;
use crate::LOGGER_PREFIX;
use colored::*;
use indicatif::{MultiProgress, ProgressBar, ProgressState, ProgressStyle};
use reqwest::header::CONTENT_LENGTH;
use reqwest::{Client, Response};
use std::cmp::min;
use std::ffi::OsStr;
use std::fmt::Write as ProgressWrite;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub struct Download;

pub struct DownloadOptions {
    pub url: String,                // url
    pub file_name: Option<String>,  // save download url, if null, will use filename by url
    pub timeout: Option<u64>,       // timeout, default `0`
    pub output_dir: Option<String>, // output dir
    pub overwrite: Option<bool>,    // if file exists, will overwrite
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
    /// get download filename
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

    /// get download timeout
    fn get_timeout(options: &DownloadOptions) -> u64 {
        let timeout = options.timeout;
        let download_timeout = if timeout.is_none() { TIMEOUT } else { timeout.unwrap() };
        return download_timeout;
    }

    /// get response
    async fn get_response(options: &DownloadOptions) -> Result<(Response, String), HttpError> {
        if options.url.is_empty() {
            println!("{} download url is empty !", LOGGER_PREFIX.cyan().bold());
            return Err(HttpError::Empty("download url is empty !".to_string()));
        }

        let download_file_name = Download::get_file_name(&options);
        if download_file_name.is_empty() {
            println!("{} download file name is empty, please check `url` or `file_name` !", LOGGER_PREFIX.cyan().bold());
            return Err(HttpError::Empty("download file name is empty, please check `url` or `file_name` !".to_string()));
        }

        let timeout = Download::get_timeout(&options);
        let client;
        if timeout <= 0 {
            client = Client::builder().build().map_err(|err| HttpError::CreateClientError(Box::new(err)))?;
        } else {
            client = Client::builder().timeout(Duration::new(timeout, 0)).build().map_err(|err| HttpError::CreateClientError(Box::new(err)))?;
        }

        let response = client.get(options.url.clone()).send().await.map_err(|err| HttpError::SendError(Box::new(err)))?;
        Ok((response, download_file_name))
    }

    /// get output file path
    fn get_output_file(options: &DownloadOptions, download_file_name: &str) -> PathBuf {
        let mut output_file_path = PathBuf::new();
        if options.output_dir.is_some() {
            let output_dir = options.output_dir.clone().unwrap();
            output_file_path.push(&output_dir);
        }

        output_file_path.push(&download_file_name);
        return output_file_path;
    }

    /// download file, include progress bar
    pub async fn download(options: DownloadOptions, progress: Option<&MultiProgress>) -> Result<DownloadResult, HttpError> {
        let mut result = DownloadResult::default();
        result.url = options.url.clone();
        result.dir = options.output_dir.clone().unwrap_or(String::new());

        let (mut response, download_file_name) = Download::get_response(&options).await?;
        result.file_name = download_file_name.clone();

        if !response.status().is_success() {
            println!("{} download file {} failed with status code: {}", LOGGER_PREFIX.cyan().bold(), &download_file_name.cyan().bold(), response.status());
            return Ok(result);
        }

        // get file size
        let content_length = response.headers().get(CONTENT_LENGTH).and_then(|value| value.to_str().ok()).and_then(|value| value.parse::<u64>().ok()).unwrap_or(0);

        if progress.is_none() {
            println!("{} file: {} content length: {}", LOGGER_PREFIX.cyan().bold(), &download_file_name, content_length);
        }

        if content_length == 0 {
            println!("{} url: {} the file on the server is empty !", LOGGER_PREFIX.cyan().bold(), &options.url);
            return Ok(result);
        }

        let output_file_path = Download::get_output_file(&options, &download_file_name);
        if progress.is_none() {
            println!("{} download file path: {}", LOGGER_PREFIX.cyan().bold(), output_file_path.as_path().to_string_lossy().to_string());
        }

        let overwrite = if options.overwrite.is_none() { true } else { options.overwrite.unwrap() };
        let mut has_need_download = true;

        // judge file is downloaded
        if output_file_path.exists() {
            let size = fs::metadata(&output_file_path).unwrap().len();
            if size == content_length {
                // download success
                has_need_download = overwrite
            }
        }

        if !has_need_download {
            if progress.is_none() {
                println!("{} file has exists, skip !", LOGGER_PREFIX.cyan().bold());
            }
            result.success = true;
            return Ok(result);
        }

        let file = match File::create(&output_file_path) {
            Ok(file) => Some(file),
            Err(err) => {
                println!("{} create file {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), output_file_path.as_path().to_string_lossy().to_string().red().bold(), err);
                None
            }
        };

        if file.is_none() {
            return Ok(result);
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
        pb.set_style(
            ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({msg}) ({eta})")
                .unwrap()
                .with_key("msg", move |_state: &ProgressState, w: &mut dyn ProgressWrite| write!(w, "{}", download_file_name_clone).unwrap())
                .with_key("eta", |state: &ProgressState, w: &mut dyn ProgressWrite| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
                .progress_chars("#>-"),
        );

        // download
        while let Some(chunk) = response.chunk().await.unwrap() {
            let chunk_size = chunk.len() as u64;
            downloaded_size += chunk_size;

            let flag = match file.write_all(&chunk) {
                Ok(_) => true,
                Err(err) => {
                    println!("{} write to file {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), &download_file_name.red().bold(), err);
                    false
                }
            };

            if !flag {
                return Ok(result);
            }

            // calculate download speed
            let elapsed_time = time.elapsed().as_secs_f64();
            if elapsed_time >= 1.0 {
                download_speed = (downloaded_size as f64 / elapsed_time) / 1_000.0; // kbps
                time = Instant::now();
            }

            // update progress bar
            pb.set_position(min(downloaded_size, content_length));
        }

        pb.finish_with_message(" ");
        file.sync_all().map_err(|err| HttpError::Error(Box::new(err)))?;

        if progress.is_none() {
            println!("{} download file {} successfully !", LOGGER_PREFIX.cyan().bold(), &download_file_name.cyan().bold());
        }

        result.success = true;
        return Ok(result);
    }
}
