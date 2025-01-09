pub mod client;
pub mod download;
pub mod options;

use crate::download::{Download, DownloadOptions, DownloadResult};
use crate::options::HttpError;
use client::HttpClient;
use colored::*;
use indicatif::MultiProgress;
use options::HttpResponse;
use options::Options;

const LOGGER_PREFIX: &str = "[Http Request]: ";

/// form data options
pub type HttpFormData = reqwest::blocking::multipart::Form;

/// send
/// is_form_submit: use form submit
pub async fn client_send(opts: Options, is_form_submit: bool) -> Result<HttpResponse, HttpError> {
    let response: Result<HttpResponse, HttpError> = HttpClient::send(opts, is_form_submit).await;
    // println!("{} response: {:#?}", LOGGER_PREFIX.cyan().bold(), response);
    return response;
}

/// send by form-data
pub fn client_send_form_data(opts: Options) -> Result<HttpResponse, HttpError> {
    let response: Result<HttpResponse, HttpError> = HttpClient::send_form_data(opts);
    // println!("{} response: {:#?}", LOGGER_PREFIX.cyan().bold(), response);
    return response;
}

/// download
pub async fn download(options: DownloadOptions, progress: Option<&MultiProgress>) -> Result<DownloadResult, HttpError> {
    return Download::download(options, progress).await;
}
