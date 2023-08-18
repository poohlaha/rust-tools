//! 公共类
use crate::LOGGER_PREFIX;
use colored::*;
use std::{fs, io};
use std::fs::File;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use fs_extra::copy_items;
use fs_extra::file::move_file;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipWriter};
pub struct Utils;

impl Utils {
    /// 执行命令
    pub fn exec_command(command: &str) -> bool {
        if command.is_empty() {
            println!("{} command is empty !", LOGGER_PREFIX.cyan().bold());
            return false;
        }

        let output: Result<Output, io::Error>;
        let _command = command.clone().replace("\n", " && ");

        // windows 通过 cmd /C 执行多条命令: cd c:\\usr\\local\\nginx\\sbin/ && nginx
        #[cfg(target_os = "windows")]
        {
            println!("{} exec command:\n {}", LOGGER_PREFIX.cyan().bold(), _command.green().bold());
            output = Command::new("cmd").args(&["/C", &_command]).output();
        }

        // linux|macos 通过 shell -c 执行多条命令: cd /usr/local/nginx/sbin/\n./nginx
        #[cfg(target_os = "macos")]
        {
            println!("{} exec command:\n {}", LOGGER_PREFIX.cyan().bold(), command.green().bold());
            output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
        }

        #[cfg(target_os = "linux")]
        {
            println!("{} exec command:\n {}", LOGGER_PREFIX.cyan().bold(), command.green().bold());
            output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
        }

        let output = match output {
            Ok(output) => Some(output),
            Err(err) => {
                println!("{} exec command error: {:?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        let mut flag = false;
        if let Some(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout).to_string();
                for line in output_str.lines() {
                    println!("{}", line);
                }
                flag = true;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("{} exec command error:\n", LOGGER_PREFIX.cyan().bold());
                for line in stderr.lines() {
                    println!("{}", line);
                }
                flag = false;
            }
        }

        return flag;
    }

    // 生成 zip
    pub fn generate_zip(source_dir: &str, output_dir: &str) -> bool {
        let source_path = Path::new(source_dir);
        let output_path = Path::new(output_dir);

        // 获取相对路径
        let source_relative_path = source_path.strip_prefix(output_path.parent().unwrap())
            .unwrap_or(source_path)
            .to_path_buf();

        println!("{} generate zip source path: {}", LOGGER_PREFIX.cyan().bold(), source_dir.cyan().bold());
        println!("{} generate zip output path: {}", LOGGER_PREFIX.cyan().bold(), output_dir.cyan().bold());
        println!("{} source relative path: {}", LOGGER_PREFIX.cyan().bold(), source_relative_path.as_path().to_str().unwrap_or("").cyan().bold());

        let file = match File::create(output_dir) {
            Ok(file) => Some(file),
            Err(err) => {
                println!("{} create file error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if file.is_none() {
            return false
        }

        let file = file.unwrap();
        let mut zip = ZipWriter::new(file);
        let options = FileOptions::default().compression_method(CompressionMethod::Stored).unix_permissions(0o755);
        let flag = match Self::add_directory_to_zip(&mut zip, &source_path, &source_relative_path.as_path(), &options) {
            Ok(_) => true,
            Err(err) => {
                println!("{} zip file error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };

        return match zip.finish(){
            Ok(_) => flag,
            Err(err) => {
                println!("{} close zip error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };
    }

    fn add_directory_to_zip(
        zip: &mut ZipWriter<File>,
        source_path: &Path,
        relative_path: &Path,
        options: &FileOptions,
    ) -> io::Result<()> {
        for entry in fs::read_dir(source_path)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = relative_path.join(entry.file_name());

            if path.is_file() {
                // 添加文件到压缩包
                zip.start_file(file_name.to_str().unwrap(), *options)?;
                let file_content = fs::read(path)?;
                zip.write_all(&file_content)?;
            } else if path.is_dir() {
                // 递归添加子目录及其内容到压缩包
                Self::add_directory_to_zip(zip, &path, &file_name, options)?;
            }
        }

        Ok(())
    }

    // 获取路径
    pub fn get_path(root_dir: &str, str_arr: &[&str]) -> String {
        let mut project_path = PathBuf::from(&root_dir);
        for str in str_arr {
            project_path.push(str);
        }

        return String::from(project_path.as_path().to_str().unwrap());
    }

    /// 移动文件
    pub fn file_move(source_dir: &str, dest_dir: &str) -> bool {
        let copy_options = fs_extra::file::CopyOptions::new();
        if let Err(err) = move_file(source_dir, dest_dir, &copy_options) {
            println!("{} Failed to move file {}", LOGGER_PREFIX.cyan().bold(), err);
            return false;
        }

        return true;
    }

    // 判断路径是否存在, 不存在则创建
    pub fn create_dir(root_dir: &str, str_arr: &[&str]) -> bool {
        let project_path = Self::get_path(&root_dir, &str_arr);

        // 路径不存在
        if !Path::new(&project_path).exists() {
            if let Err(err) = fs::create_dir_all(project_path) {
                println!("{} Failed to create directory: {}, error: {:?}", LOGGER_PREFIX.cyan().bold(), root_dir.magenta().bold(), err);
                return false;
            } else {
                // println!("{} create dir {} success", LOGGER_PREFIX.cyan().bold(), root_dir.magenta().bold());
                return true;
            }
        }

        return true;
    }

    /// 拷贝目录
    pub fn copy_dir(source_dir: &str, dest_dir: &str) -> bool {
        if !Path::new(&source_dir).exists() {
            println!("{} dir {} not exists, copy failed !", LOGGER_PREFIX.cyan().bold(), source_dir.magenta().bold());
            return false;
        }

        Self::create_dir(&dest_dir, &[]);

        let mut from_paths = Vec::new();
        from_paths.push(source_dir);

        let options = fs_extra::dir::CopyOptions::new();
        if let Err(err) = copy_items(&from_paths, dest_dir, &options) {
            println!("{} Failed to copy folder {}", LOGGER_PREFIX.cyan().bold(), err);
            return false;
        }

        return true;
    }
}

