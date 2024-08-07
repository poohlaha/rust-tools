//! 文件操作

use crate::error::Error;
use crypto_hash::{hex_digest, Algorithm};
use log::{error, info};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub struct FileHandler;

impl FileHandler {
    /// 创建目录
    pub fn create_dirs(dir: &str) -> Result<PathBuf, String> {
        if dir.is_empty() {
            error!("create dirs failed, `name` is empty !");
            return Err(Error::convert_string("create dirs failed, `name` is empty !"));
        }

        let path = PathBuf::from(dir);

        // 存在则直接返回
        if path.exists() {
            return Ok(path);
        }

        // 不存在则创建
        fs::create_dir_all(&path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(path)
    }

    /// 打开文件
    pub fn open_file(file_path: &str) -> Result<File, String> {
        let file = File::open(&file_path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(file)
    }

    /// 读取文件 - 字符串
    pub fn read_file_string(file_path: &str) -> Result<String, String> {
        let mut file = Self::open_file(file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(contents)
    }

    /// 读取文件 - 字节数组
    pub fn read_file_buffer(file_path: &str) -> Result<Vec<u8>, String> {
        let mut file = Self::open_file(file_path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(buffer)
    }

    /// 写入每一行
    pub fn write_file_string_pre_line(file_path: &str, pre_str: &str) -> Result<(), String> {
        if !Path::new(file_path).exists() {
            Self::write_to_file_when_clear(file_path, &pre_str)
        } else {
            let mut content = Self::read_file_string(file_path)?;
            content.push_str("\r\n");
            content.push_str(pre_str);
            Self::write_to_file_when_clear(file_path, &content)
        }
    }

    /// 清空文件并写入新的内容
    pub fn write_to_file_when_clear(file_path: &str, content: &str) -> Result<(), String> {
        // 打开文件以进行覆盖写入
        let mut file = File::create(&file_path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        file.write_all(content.as_bytes()).map_err(|err| Error::Error(err.to_string()).to_string())?;
        file.flush().unwrap(); // 刷新文件缓冲
        file.sync_all().unwrap(); // 写入磁盘
        drop(file); // 自动关闭文件
        Ok(())
    }
}

/// 文件操作
impl FileHandler {
    /// 删除文件目录
    pub fn delete_dirs(paths: Vec<String>) -> Result<(), String> {
        fs_extra::remove_items(&paths).map_err(|err| Error::Error(err.to_string()).to_string())
    }

    /// 删除文件
    pub fn delete_file(file_path: &str) -> Result<(), String> {
        fs_extra::file::remove(file_path).map_err(|err| Error::Error(err.to_string()).to_string())
    }

    /// 移动目录
    pub fn move_dirs(paths: Vec<String>, target_dir: &str) -> Result<(), String> {
        let mut options = fs_extra::dir::CopyOptions::new();
        options = options.overwrite(true);
        fs_extra::move_items(&paths, target_dir, &options).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(())
    }

    /// 移动文件
    pub fn move_files(paths: Vec<String>, target_dir: &str) -> Result<(), String> {
        let mut options = fs_extra::file::CopyOptions::new();
        options = options.overwrite(true);

        for path in paths.iter() {
            let filename = Path::new(path).file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
            let target_file = PathBuf::from(target_dir).join(filename);
            fs_extra::file::move_file(&path, target_file.to_string_lossy().to_string().as_str(), &options).map_err(|err| Error::Error(err.to_string()).to_string())?;
        }

        Ok(())
    }

    /// 重命名文件或文件夹
    pub fn rename(file: &str, dest_file: &str) -> Result<(), String> {
        fs::rename(file, dest_file).map_err(|err| Error::Error(err.to_string()).to_string())?;
        Ok(())
    }

    /// 读取文件
    pub fn read_dir<P: AsRef<Path>>(path: P) -> Result<(Vec<String>, Vec<String>), String> {
        let entries = fs::read_dir(path).map_err(|err| Error::Error(err.to_string()).to_string())?;
        let mut files: Vec<String> = Vec::new();
        let mut directories: Vec<String> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|err| Error::Error(err.to_string()).to_string())?;
            let file_path = entry.path();
            let file_name = file_path.file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();

            if file_path.is_dir() {
                directories.push(file_name)
            } else {
                files.push(file_name)
            }
        }

        Ok((directories, files))
    }

    /// 获取文件 hash 值
    pub fn get_file_hash(file_path: &str) -> Result<String, String> {
        let path = Path::new(file_path);
        // 文件不存在
        if !path.exists() {
            info!("get file hash failed, file path: `{}` is not exists!", file_path);
            return Ok(String::new());
        }

        let mut file = File::open(path).map_err(|err| {
            let msg = format!("get file hash failed, open file `{}` error: {:#?}", file_path, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).map_err(|err| {
            let msg = format!("get file hash failed, read file `{}` error: {:#?}", file_path, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        Ok(hex_digest(Algorithm::SHA256, &buffer))
    }
}
