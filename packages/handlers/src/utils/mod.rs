//! Utils 类

use std::{fs};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use log::info;
use zip::write::{FileOptions, SimpleFileOptions};
use zip::{CompressionMethod, ZipWriter};
use crate::error::Error;
use crate::file::FileHandler;

pub struct Utils;

impl Utils {

    /// 获取年月日
    pub fn get_date(format: Option<String>) -> String {
        let mut date_format = String::from("%Y-%m-%d %H:%M:%S");
        if let Some(format) = format {
            date_format = format
        }

        chrono::Local::now().format(&date_format).to_string()
    }

    /// 生成 zip 压缩包
    pub fn generate_zip(dir: &str, output_file: &str) -> Result<bool, String> {
        if !output_file.ends_with(".zip") {
            return Err(Error::convert_string("generate zip failed, `ouput_dir` is not a zip file !"));
        }

        let path = Path::new(output_file);
        if path.exists() {
            FileHandler::delete_file(output_file)?;
        }

        // 获取相对路径
        let source_dir_path = Path::new(dir);
        let relative_path = source_dir_path.strip_prefix(path.parent().unwrap()).unwrap_or(path).to_path_buf();
        let file = File::create(&path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Stored).unix_permissions(0o777);
        Self::add_directory_to_zip(&mut zip, &source_dir_path, &relative_path, &options)?;
        zip.finish().map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(true)
    }

    /// 添加到 zip 包中
    fn add_directory_to_zip(zip: &mut ZipWriter<File>, source_path: &Path, relative_path: &Path, options: &FileOptions<()>) -> Result<(), String> {
        let entries = fs::read_dir(source_path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        for entry in entries {
            let entry = entry.map_err(|err| Error::Error(err.to_string()).to_string())?;
            let path = entry.path();
            let file_name = relative_path.join(entry.file_name());

            if path.is_file() {
                // 添加文件到压缩包x
                zip.start_file(file_name.to_str().unwrap(), *options).map_err(|err| Error::Error(err.to_string()).to_string())?;
                let file_content = fs::read(path).map_err(|err| Error::Error(err.to_string()).to_string())?;
                zip.write_all(&file_content).map_err(|err| Error::Error(err.to_string()).to_string())?;
            } else if path.is_dir() {
                // 递归添加子目录及其内容到压缩包
                Self::add_directory_to_zip(zip, &path, &file_name, options).map_err(|err| Error::Error(err.to_string()).to_string())?;
            }
        }

        Ok(())
    }



    /// 版权所有
    pub fn copyright() {
        info!(" ");
        info!(" ---------------- Copyright ----------------- ");
        info!("|                                             |");
        info!("| Author: poohlaha                            |");
        info!("| Github: https://github.com/poohlaha         |");
        info!("|                                             |");
        info!(" ----------------- Copyright ----------------- ");
        info!(" ");
    }
}
