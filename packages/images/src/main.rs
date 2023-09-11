use crate::compressor::{Compressor, CompressorArgs};

mod compressor;
mod factor;
mod img;

const LOGGER_PREFIX: &str = "[Images]: ";

fn main() {
    let args = CompressorArgs {
        factor: None,
        origin: "/usr/local/www/images".to_string(),
        dest: "/usr/local/www/outputs".to_string(),
        thread_count: None,
        need_convert_format: false
    };

    let compressor = Compressor::new(args);
    compressor.compress();
}