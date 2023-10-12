//!  执行 linux 命令, 文件上传等操作

use crate::config::{Server, Upload};
use crate::LOGGER_PREFIX;
use chrono::Local;
use colored::*;
use crypto_hash::{hex_digest, Algorithm};
use fs_extra::dir;
use fs_extra::move_items;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use ssh2::{FileStat, Session};
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;
use crate::utils::Utils;

pub struct Sftp {
    server: Server,
    upload: Upload,
}

#[derive(Debug, Default)]
pub struct SftpResult {
    pub dir: String,
    pub file_name: String,
    pub zip_file_name: String,
    pub file_count: u64,
    pub file_list: Vec<String>,
    pub need_increment: bool,
    pub success: bool,
}

impl Sftp {
    pub fn new(mut server: Server, mut upload: Upload) -> Self {
        if server.is_empty() {
            server = Server::default()
        }

        if upload.is_empty() {
            upload = Upload::default()
        }
        return Self { server, upload };
    }

    /// 连接服务器, 处理逻辑
    pub fn send(&mut self) -> SftpResult {
        // 处理输出目录, 压缩成 .zip 文件
        let (server_file_name, server_zip_file_name) = self.hand_output_dir();
        return self.exec(&server_file_name, &server_zip_file_name);
    }

    /// 连接服务器, 处理逻辑
    pub fn exec(&mut self, server_file_name: &str, server_zip_file_name: &str) -> SftpResult {
        let mut result = SftpResult::default();
        let server = &self.server;
        if server.is_empty() {
            println!("{} one of `host`、`port`、`username` and `password` items is empty !", LOGGER_PREFIX.cyan().bold());
            return result;
        }

        // 拼接服务器地址
        let address = format!("{}:{}", &server.host, server.port);
        let tcp = match TcpStream::connect_timeout(&SocketAddr::from_str(&address).unwrap(), Duration::from_secs(server.timeout)) {
            Ok(tcp) => Some(tcp),
            Err(err) => {
                println!("{} connect to {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), address.clone().red().bold(), err);
                None
            }
        };

        if tcp.is_none() {
            return result;
        }

        // 创建 session
        let session = match Session::new() {
            Ok(session) => Some(session),
            Err(err) => {
                println!("{} create session error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if session.is_none() {
            return result;
        }

        let mut session = session.unwrap();
        let tcp = tcp.unwrap();

        // 设置用户名和密码
        session.set_tcp_stream(tcp);
        session.handshake().unwrap();
        match session.userauth_password(&self.server.username, &self.server.password) {
            Ok(_) => {}
            Err(err) => {
                println!("{} username or password is error, {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                return result;
            }
        }

        // 认证用户名密码是否正确
        if !session.authenticated() {
            println!("{} authentication server: {} failed !", LOGGER_PREFIX.cyan().bold(), address.clone().red().bold());
        }

        println!("{} connect {} success !", LOGGER_PREFIX.cyan().bold(), address.clone().magenta().bold());

        // 处理文件上传
        if !server_file_name.is_empty() && !server_zip_file_name.is_empty() {
            let sftp_result = self.upload(&session, server_file_name, server_zip_file_name, &self.upload);
            result = sftp_result;
        } else {
            // 直接执行命令
            let cmds = &self.upload.cmds;
            if cmds.is_empty() {
                return result;
            }

            let success = self.exec_command(&session, cmds);
            result.success = success
        }

        // 关闭 session
        let flag = match session.disconnect(None, "Bye bye !", None) {
            Ok(_) => result.success,
            Err(err) => {
                println!("{} close session error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };

        if flag {
            println!("{} upload success !", LOGGER_PREFIX.cyan().bold());
            result.dir = String::from(&self.upload.dir);
            result.file_name = String::from(server_file_name);
            result.zip_file_name = String::from(server_zip_file_name);
            result.success = true;
            return result;
        }

        println!("{} upload failed !", LOGGER_PREFIX.cyan().bold());
        return result;
    }

    /// 文件上传
    fn upload(&self, session: &Session, server_file_name: &str, server_zip_file_name: &str, upload: &Upload) -> SftpResult {
        let sftp_result = SftpResult::default();

        let mut file_path = PathBuf::from(&upload.dir);
        file_path.push(server_zip_file_name);

        let local_file_path = file_path.as_path().to_str().unwrap(); // 本地 zip 文件路径

        // 打开本地文件
        let file = match File::open(Path::new(local_file_path)) {
            Ok(file) => Some(file),
            Err(err) => {
                println!("{} open local file error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if file.is_none() {
            println!("{} upload file: {} failed !", LOGGER_PREFIX.cyan().bold(), local_file_path.clone().red().bold());
            return sftp_result;
        }

        let mut file = file.unwrap();
        let mut buffer = Vec::new();
        let size = match file.read_to_end(&mut buffer) {
            Ok(size) => Some(size),
            Err(err) => {
                println!("{} read file: {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), local_file_path.clone().magenta().bold(), err);
                None
            }
        };

        if size.is_none() {
            return sftp_result;
        }

        // 创建 sftp
        let sftp = match session.sftp() {
            Ok(sftp) => Some(sftp),
            Err(err) => {
                println!("{} use session sftp error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if sftp.is_none() {
            return sftp_result;
        }

        let sftp = sftp.unwrap();
        let server_dir = &upload.server_dir;
        let server_file = format!("{}{}", server_dir, server_zip_file_name);
        println!("{} server file path: {}", LOGGER_PREFIX.cyan().bold(), server_file.clone().green().bold());

        // 创建远程文件
        let remote_file = match sftp.create(Path::new(&server_file)) {
            Ok(file) => Some(file),
            Err(err) => {
                println!("{} use session sftp create error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if remote_file.is_none() {
            println!("{} upload file: {} failed !", LOGGER_PREFIX.cyan().bold(), local_file_path.clone().red().bold());
            return sftp_result;
        }

        let mut remote_file = remote_file.unwrap();

        println!("{} uploading file: {}", LOGGER_PREFIX.cyan().bold(), local_file_path.clone().green().bold());

        // 进行 process bar
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_style(ProgressStyle::with_template("{spinner:.blue} {msg}").unwrap().tick_strings(&["▹▹▹▹▹", "▸▹▹▹▹", "▹▸▹▹▹", "▹▹▸▹▹", "▹▹▹▸▹", "▹▹▹▹▸", "▪▪▪▪▪"]));
        pb.set_message("Uploading...");

        // 写入远程文件
        let success = match remote_file.write_all(&buffer) {
            Ok(_) => Some(1),
            Err(err) => {
                println!("{} write local file to remote file error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if success.is_none() {
            println!("{} upload file: {} failed !", LOGGER_PREFIX.cyan().bold(), local_file_path.clone().red().bold());
            return sftp_result;
        }

        pb.finish_with_message("Upload Success !");

        println!("{} upload file success, file path: {}", LOGGER_PREFIX.cyan().bold(), server_file.clone().green().bold());

        // 解压 zip 包, 删除原来目录，用新的目录覆盖
        return self.uncompress(session, &sftp, server_file_name, server_zip_file_name, upload);
    }

    /// 获取远程解压服务器下的文件夹
    fn get_remote_dir(&self, entries: &Option<Vec<(PathBuf, FileStat)>>) -> Vec<String> {
        let mut directories: Vec<String> = Vec::new();
        if entries.is_none() {
            return directories;
        }

        let entries = entries.clone().unwrap();
        for (path, file_stat) in entries.iter() {
            if file_stat.is_dir() {
                directories.push(path.to_string_lossy().to_string());
            }
        }

        return directories;
    }

    /// 解压 zip 包, 删除原来目录，用新的目录覆盖
    fn uncompress(&self, session: &Session, sftp: &ssh2::Sftp, server_file_name: &str, server_zip_file_name: &str, upload: &Upload) -> SftpResult {
        let server_dir = &upload.server_dir;
        let mut sftp_result = SftpResult::default();
        println!("{} begin to unzip {} ...", LOGGER_PREFIX.cyan().bold(), format!("{}{}", server_dir, server_zip_file_name).green().bold());

        // 解压
        let server_zip_file_stem: &str = Path::new(server_zip_file_name).file_stem().unwrap().to_str().unwrap_or(""); // 文件前缀
        let unzip_dir = Utils::get_path(server_dir, &[&server_zip_file_stem]);
        let server_project_dir_stem: &str = Path::new(server_file_name).file_stem().unwrap().to_str().unwrap_or(""); // 文件前缀
        let server_project_dir = Utils::get_path(server_dir, &[&server_project_dir_stem]); // 原来项目目录

        let commands: Vec<String> = vec![
            format!("cd {}", server_dir),                               // cd 到服务器目录
            format!("unzip {} -d {}", server_zip_file_name, unzip_dir), // unzip 到指定目录 -o 为强制覆盖
        ];

        let success = self.exec_command(session, &commands);
        if !success {
            println!("{} unzip {} failed !", LOGGER_PREFIX.cyan().bold(), format!("{}{}", server_dir, server_zip_file_name).red().bold());
            self.publish_delete(session, server_dir, &unzip_dir, server_zip_file_name);
            return sftp_result;
        }

        println!("{} unzip {} success !", LOGGER_PREFIX.cyan().bold(), format!("{}{}", server_dir, server_zip_file_name).green().bold());

        println!("{} begin to {} version ...", LOGGER_PREFIX.cyan().bold(), "publishing".green().bold());
        let new_entries = self.read_dir(sftp, &unzip_dir);
        // 判断 unzip 解压目录下是否只有一个目录
        let directories = self.get_remote_dir(&new_entries);

        // 获取压缩后的项目相对目录
        let zip_project_dir;
        if directories.len() == 1 {
            zip_project_dir = directories.get(0).unwrap().to_string();
        } else {
            zip_project_dir = unzip_dir.clone();
        }

        // 解压后的全路径
        let zip_project_dir_path = if directories.len() == 1 { Utils::get_path(&unzip_dir, &[&zip_project_dir]) } else { unzip_dir.clone() };

        // 先判断是否需要增量发布
        let mut need_increment = upload.need_increment;
        // new files
        let mut new_files: Vec<String> = Vec::new();
        self.read_files(sftp, &zip_project_dir_path, &mut new_files);

        let mut old_files: Vec<String> = Vec::new();
        println!("{} server project dir: {}", LOGGER_PREFIX.cyan().bold(), &server_project_dir);
        if sftp.stat(Path::new(&server_project_dir)).is_ok() {
            println!("{} read path: {} files !", LOGGER_PREFIX.cyan().bold(), &server_project_dir.cyan().bold());
            self.read_files(sftp, &server_project_dir, &mut old_files);
        } else {
            println!("{} old project dir: {} not exists !", LOGGER_PREFIX.cyan().bold(), &server_project_dir.cyan().bold());
        }

        if old_files.is_empty() {
            println!("{} old files is empty, full publish project !", LOGGER_PREFIX.cyan().bold());
            need_increment = false;
        }

        sftp_result.need_increment = need_increment;
        let success;
        if need_increment {
            // 增量发布
            let difference_files = self.compare_dir(sftp, &zip_project_dir_path, &new_files, &old_files, &server_project_dir);
            println!("{} difference files count: {}", LOGGER_PREFIX.cyan().bold(), difference_files.len().to_string().cyan().bold());
            if !difference_files.is_empty() {
                println!("{} difference files: {:#?}", LOGGER_PREFIX.cyan().bold(), difference_files);
                sftp_result.file_count = difference_files.len() as u64;
                sftp_result.file_list = difference_files.clone().iter().map(|(_, _, relative_path)| relative_path.to_string()).collect();
                success = self.incremental_publish(session, sftp, &upload.cmds, &difference_files, &new_files, &server_project_dir, &zip_project_dir_path);
            } else {
                println!("{} no difference files, publish version skip !", LOGGER_PREFIX.cyan().bold());
                // 去除没用的文件
                success = self.remove_no_used_files_in_project(session, sftp, &upload.cmds, &new_files, &server_project_dir, &zip_project_dir_path);
            }
        } else {
            // 全量发布
            let mut files: Vec<String> = Vec::new();
            self.read_files(sftp, &zip_project_dir_path, &mut files);
            sftp_result.file_count = files.len() as u64;
            println!("{} all file count: {}", LOGGER_PREFIX.cyan().bold(), sftp_result.file_count);
            success = self.full_publish(session, &upload.cmds, server_dir, &server_project_dir, &unzip_dir, server_zip_file_name, &zip_project_dir);
        }

        if !success {
            println!("{} publish version failed ...", LOGGER_PREFIX.red().bold());
            self.publish_delete(session, server_dir, &unzip_dir, server_zip_file_name);
            return sftp_result;
        }

        // 成功后删除文件夹和zip文件
        self.publish_delete(session, server_dir, &unzip_dir, server_zip_file_name);
        println!("{} {} ...", LOGGER_PREFIX.cyan().bold(), "publish version success".green().bold());
        sftp_result.success = true;
        return sftp_result;
    }

    /// 删除压缩目录和压缩包
    fn publish_delete(&self, session: &Session, server_dir: &str, unzip_dir: &str, server_zip_file_name: &str) {
        let commands: Vec<String> = vec![
            format!("cd {}", server_dir),               // cd 到服务器目录
            format!("rm -rf {}", unzip_dir),            // 删除已解压目录
            format!("rm -rf {}", server_zip_file_name), // 删除压缩包
        ];
        self.exec_command(session, &commands);
    }

    /// 增量发布
    fn incremental_publish(&self, session: &Session, sftp: &ssh2::Sftp, cmds: &Vec<String>, difference_files: &Vec<(String, String, String)>, new_files: &Vec<String>, server_project_dir: &str, zip_project_dir_path: &str) -> bool {
        let mut commands: Vec<String> = Vec::new();
        for (new_file, old_file, file_path) in difference_files.iter() {
            let new_file_path = Path::new(new_file);

            // 判断文件是不是绝对路径
            if new_file_path.is_absolute() {
                let mut project_path = PathBuf::from(server_project_dir);
                project_path.push(file_path);
                commands.push(format!("rm -rf {}", old_file)); // 1. 删除旧的文件
                commands.push(format!("cp {} {}", new_file, project_path.as_path().to_string_lossy().to_string()));
            // 2. 拷贝新文件到原来目录
            } else {
                println!("{} file {} not a absolute address !", LOGGER_PREFIX.cyan().bold(), new_file.red().bold());
            }
        }

        println!("{} commands: {:#?}", LOGGER_PREFIX.cyan().bold(), commands);
        let success = self.exec_command(session, &commands);
        if !success {
            return false;
        }

        // 去除旧项目中没用的文件
        return self.remove_no_used_files_in_project(session, sftp, cmds, new_files, server_project_dir, &zip_project_dir_path);
    }

    /// 去除旧项目中没用的文件
    fn remove_no_used_files_in_project(&self, session: &Session, sftp: &ssh2::Sftp, cmds: &Vec<String>, new_files: &Vec<String>, server_project_dir: &str, zip_project_dir_path: &str) -> bool {
        // old files
        let mut old_files: Vec<String> = Vec::new();
        self.read_files(sftp, server_project_dir, &mut old_files);

        let mut commands: Vec<String> = Vec::new();

        // 判断文件是否存在
        for file in old_files.iter() {
            // 不带项目的路径
            let file_relative_path = Path::new(&file).strip_prefix(server_project_dir).unwrap().to_str().unwrap();
            let find_file: Option<&String> = new_files.iter().find(|f| {
                let new_file_relative_path = Path::new(&f).strip_prefix(zip_project_dir_path).unwrap().to_str().unwrap();
                return new_file_relative_path.to_string() == file_relative_path.clone().to_string();
            });

            if find_file.is_none() {
                let mut project_path = PathBuf::from(server_project_dir);
                project_path.push(file_relative_path);
                let delete_file = project_path.as_path().to_string_lossy().to_string();
                println!("{} file {} is not used, will be deleted !", LOGGER_PREFIX.cyan().bold(), &delete_file.cyan().bold());
                commands.push(format!("rm -rf {}", delete_file));
            }
        }

        if commands.is_empty() {
            println!("{} no used file to be delete !", LOGGER_PREFIX.cyan().bold());
        }

        // 合并服务端命令
        let cmds: Vec<String> = cmds.iter().map(|cmd| cmd.to_string()).collect();
        commands.extend(cmds);
        if commands.is_empty() {
            return true;
        }

        return self.exec_command(session, &commands);
    }

    /// 全量发布
    fn full_publish(&self, session: &Session, cmds: &Vec<String>, server_dir: &str, server_project_dir: &str, unzip_dir: &str, server_zip_file_name: &str, mv_zip_project_dir: &str) -> bool {
        // 删除原来的项目, 更改解压后的目录为原来目录
        let mut commands: Vec<String> = vec![
            format!("cd {}", server_dir),             // cd 到服务器目录
            format!("rm -rf {}", server_project_dir), // 删除原来的目录
        ];

        commands.push(format!("mv {} {}", mv_zip_project_dir, &server_dir)); // 移动
        commands.push(format!("rm -rf {}", unzip_dir)); // 删除压缩目录
        commands.push(format!("rm -rf {}", server_zip_file_name)); // 删除压缩包
                                                                   // 合并服务端命令
        let cmds: Vec<String> = cmds.iter().map(|cmd| cmd.to_string()).collect();
        commands.extend(cmds);
        return self.exec_command(session, &commands);
    }

    /// 读取服务器目录
    fn read_dir(&self, sftp: &ssh2::Sftp, dir: &str) -> Option<Vec<(PathBuf, FileStat)>> {
        return match sftp.readdir(Path::new(dir)) {
            Ok(entries) => Some(entries),
            Err(err) => {
                println!("{} session sftp read dir {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), dir.magenta().bold(), err);
                None
            }
        };
    }

    /// 判断文件名，文件不带hash值后的文件名是否一致
    fn judge_file_same(&self, file1: &str, file2: &str, file1_prefix_dir: &str, file2_prefix_dir: &str) -> bool {
        let path1 = Path::new(file1.trim());
        let path2 = Path::new(file2.trim());
        // println!("file1: {file1}, file2: {file2}");

        // 1. 比较路径相对路径是否一致
        let path1_stem: &str = path1.parent().unwrap().to_str().unwrap(); // 文件前缀
        let path2_stem: &str = path2.parent().unwrap().to_str().unwrap(); // 文件前缀
        let file1_relative_path = path1_stem.replace(file1_prefix_dir, "");
        let file2_relative_path = path2_stem.replace(file2_prefix_dir, "");
        // println!("path1_stem: {path1_stem}, path2_stem: {path2_stem}");
        // println!("file1_relative_path: {file1_relative_path}, file2_relative_path: {file2_relative_path}");
        if file1_relative_path.to_lowercase().to_string() != file2_relative_path.to_lowercase().to_string() {
            // println!("path not same");
            return false;
        }

        // 2. 比较相对路径文件是否一致
        let file1_relative_file_path = file1.replace(file1_prefix_dir, "");
        let file2_relative_file_path = file2.replace(file2_prefix_dir, "");
        // println!("file1_relative_file_path: {file1_relative_file_path}, file2_relative_file_path: {file2_relative_file_path}");
        if file1_relative_file_path.to_string() == file2_relative_file_path.to_string() {
            return true;
        }

        // 2. 比较文件名
        let file1_name = path1.file_name().unwrap().to_string_lossy().to_string();
        let file2_name = path2.file_name().unwrap().to_string_lossy().to_string();
        let file1_extension = path1.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
        let file2_extension = path2.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");

        // 2.1 判断文件是否以 hash 值开头, 以 hash 开头则需要替换
        let success = self.math_filename_start_with_hash(&file1_name, &file2_name, file1_extension, file2_extension);
        if success {
            return false;
        }

        // 2.2 判断文件去除 hash 值后是否一致
        return self.judge_filename_hash(&file1_name, &file2_name);
    }

    /// 检验文件名中去除包含的hash值是否一样
    fn judge_filename_hash(&self, file1_name: &str, file2_name: &str) -> bool {
        let hash_pattern = Regex::new(r"\.[0-9a-fA-F.]+").unwrap();
        if !hash_pattern.is_match(file1_name) || !hash_pattern.is_match(file2_name) {
            return false;
        }

        let new_filename1 = hash_pattern.replace(file1_name, ".").to_string();
        let new_filename2 = hash_pattern.replace(file2_name, ".").to_string();
        return new_filename1 == new_filename2;
    }

    // 判断文件是否以 hash 值开头, 以 hash 开头则需要替换
    fn math_filename_start_with_hash(&self, file1_name: &str, file2_name: &str, file1_extension: &str, file2_extension: &str) -> bool {
        let hash_pattern = Regex::new(r"^[0-9a-fA-F.]+").unwrap();
        let match_name1 = hash_pattern.replace(&file1_name, "");
        let match_name2 = hash_pattern.replace(&file2_name, "");
        return match_name1 == file1_extension && match_name2 == file2_extension;
    }

    // 判断文件名中是否带有hash
    fn math_filename_hash(&self, file1_name: &str, file2_name: &str, file1_extension: &str, file2_extension: &str) -> bool {
        let success = self.math_filename_start_with_hash(file1_name, file2_name, file1_extension, file2_extension);
        if success {
            return true;
        }

        let hash_pattern = Regex::new(r"\.[0-9a-fA-F.]+").unwrap();
        if !hash_pattern.is_match(file1_name) || !hash_pattern.is_match(file2_name) {
            return false;
        }

        return true;
    }

    /// 获取远程文件的 hash 值
    fn get_remote_file_hash(&self, sftp: &ssh2::Sftp, file_path: &str) -> String {
        let file = match sftp.open(Path::new(file_path)) {
            Ok(file) => Some(file),
            Err(err) => {
                println!("{} open file {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), file_path.red().bold(), err);
                None
            }
        };

        if file.is_none() {
            return String::new();
        }

        let mut file = file.unwrap();
        let mut buffer = Vec::new();
        let size = match file.read_to_end(&mut buffer) {
            Ok(size) => Some(size),
            Err(err) => {
                println!("{} open read {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), file_path.red().bold(), err);
                None
            }
        };

        if size.is_none() {
            return String::new();
        }

        return hex_digest(Algorithm::SHA256, &buffer);
    }

    /// 比较两个目录文件是否一致
    fn compare_dir(&self, sftp: &ssh2::Sftp, zip_project_dir: &str, new_files: &Vec<String>, old_files: &Vec<String>, server_project_dir: &str) -> Vec<(String, String, String)> {
        println!("{} incremental publishing {} ", LOGGER_PREFIX.cyan().bold(), server_project_dir.cyan().bold());

        // (new file path, old file path)
        let mut difference_files: Vec<(String, String, String)> = Vec::new();
        // compare
        for file in new_files.iter() {
            // 查找文件路径, 比较文件
            let find_file: Option<&String> = old_files.iter().find(|f| self.judge_file_same(f, file, server_project_dir, zip_project_dir));
            let file_relative_path = Path::new(&file).strip_prefix(zip_project_dir).unwrap().to_str().unwrap();
            // 没有找到
            if find_file.is_none() {
                difference_files.push((file.clone(), String::new(), file_relative_path.to_string()));
                continue;
            }

            let find_file = find_file.unwrap();
            let file1_name = Path::new(find_file).file_name().unwrap().to_string_lossy().to_string();
            let file2_name = Path::new(file).file_name().unwrap().to_string_lossy().to_string();
            let file1_extension = Path::new(find_file).extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
            let file2_extension = Path::new(file).extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");

            // println!("file1_name: {file1_name} file2_name: {file2_name}");
            // 在旧项目中已找到, 比较文件是否一致
            if file1_name == file2_name {
                // 判断文件名是否一致
                let has_same = self.math_filename_hash(&file1_name, &file2_name, file1_extension, file2_extension);
                if !has_same {
                    println!("{} filename {} same but it has no hash code, compare file hash !", LOGGER_PREFIX.cyan().bold(), &file_relative_path.cyan().bold());
                    let old_hash = self.get_remote_file_hash(sftp, &find_file);
                    let new_hash = self.get_remote_file_hash(sftp, file);
                    if old_hash != new_hash {
                        // println!("hash no : {} {}", file, find_file);
                        difference_files.push((file.clone(), find_file.clone(), file_relative_path.to_string()));
                    }
                }
            } else {
                // println!("different file ...");
                // println!("different file: {} {}", file, find_file);
                difference_files.push((file.clone(), find_file.clone(), file_relative_path.to_string()));
            }
        }

        println!("{} {} ...", LOGGER_PREFIX.cyan().bold(), format!("compare {} success", server_project_dir.cyan().bold()).cyan().bold());
        return difference_files;
    }

    fn read_files(&self, sftp: &ssh2::Sftp, dir: &str, files: &mut Vec<String>) {
        let entries = self.read_dir(sftp, dir);
        let entries = entries.unwrap();
        for (path, file_stat) in entries.iter() {
            let path = path.to_string_lossy().to_string();
            if file_stat.is_dir() {
                self.read_files(sftp, &path, files);
            } else {
                files.push(path);
            }
        }
    }

    /// 处理输出目录, 压缩成 .zip 文件
    pub fn hand_output_dir(&self) -> (String, String) {
        let upload = &self.upload;
        if upload.is_empty() {
            println!("{} upload props is empty, skip upload !", LOGGER_PREFIX.cyan().bold());
            return (String::new(), String::new());
        }

        let upload_dir = &upload.dir;
        let server_dir = &upload.server_dir;
        let server_file_name = &upload.server_file_name;
        if upload_dir.is_empty() {
            println!("{} upload dir is empty !", LOGGER_PREFIX.cyan().bold());
            return (String::new(), String::new());
        }

        // 判断目录是否存在, 不存在则创建
        if !Path::new(upload_dir).exists() {
            println!("{} upload dir {} is not exists !", LOGGER_PREFIX.cyan().bold(), upload_dir.magenta().bold());
            return (String::new(), String::new());
        }

        if server_dir.is_empty() {
            println!("{} upload to server dir is empty !", LOGGER_PREFIX.cyan().bold());
            return (String::new(), String::new());
        }

        // 获取文件名
        let server_file_name = self.get_file_name(upload_dir, server_file_name.clone());
        if server_file_name.is_empty() {
            return (String::new(), String::new());
        }

        println!("{} get the file name: {}", LOGGER_PREFIX.cyan().bold(), server_file_name.green().bold());

        // 文件名重新生成
        let date: String = Local::now().format("%Y%m%d%H%M%S").to_string(); // 生成时间
        let server_file_path = PathBuf::from(&server_file_name);
        // 获取文件后缀
        let extension = server_file_path.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
        let server_zip_file_name;
        if extension.is_empty() {
            server_zip_file_name = server_file_name.clone() + "-" + &date;
        } else {
            let server_file_stem: &str = server_file_path.file_stem().unwrap().to_str().unwrap_or(""); // 文件前缀
            server_zip_file_name = String::from(server_file_stem) + "-" + &date + "." + extension;
        }

        println!("{} get the server file name: {}", LOGGER_PREFIX.cyan().bold(), server_zip_file_name.green().bold());

        // 如果以 zip 结尾
        if extension.ends_with("zip") {
            let file_path = Utils::get_path(&upload_dir, &[&server_file_name]);
            let output_file_path = Utils::get_path(&upload_dir, &[&server_zip_file_name]);
            if !Path::new(&output_file_path).exists() {
                let success = Utils::file_move(&file_path, &output_file_path);
                if !success {
                    return (String::new(), String::new());
                }
            }

            return (server_file_name.clone(), server_zip_file_name.clone());
        }

        // 不是.zip 文件, 则压缩成 zip 文件
        println!("{} begin to compress dir {} to zip ...", LOGGER_PREFIX.cyan().bold(), upload_dir.magenta().bold());
        let success = self.compress_dir(upload_dir, &server_file_name, &server_zip_file_name);
        let server_zip_file_name = server_zip_file_name.clone() + ".zip";
        println!("{} end to compress dir top zip, zip name: {}", LOGGER_PREFIX.cyan().bold(), server_zip_file_name.magenta().bold());
        if success {
            return (server_file_name.clone(), server_zip_file_name.clone());
        }

        return (String::new(), String::new());
    }

    /// 获取文件名
    fn get_file_name(&self, upload_dir: &str, server_file_name: Option<String>) -> String {
        let path = Path::new(upload_dir);
        let file_name = self.get_path_file_name(path);

        // 存在文件名
        if server_file_name.is_some() {
            // 判断目录是否为空
            if file_name.is_empty() {
                return String::new();
            }

            let server_file_name = server_file_name.unwrap();
            if file_name.ends_with(".zip") {
                let file_stem = PathBuf::from(&file_name).file_stem().unwrap().to_string_lossy().to_string(); // 文件前缀
                if file_stem == server_file_name {
                    return file_name;
                }

                // 移动为名称
                let source_dir = Utils::get_path(upload_dir, &[&file_name]);
                let output_file_name = server_file_name.clone() + ".zip";
                let dest_dir = Utils::get_path(upload_dir, &[&output_file_name]);
                println!("{} source_dir: {}", LOGGER_PREFIX.cyan().bold(), source_dir);
                println!("{} dest_dir: {}", LOGGER_PREFIX.cyan().bold(), dest_dir);

                if Path::new(&dest_dir).exists() {
                    return output_file_name;
                }

                let success = Utils::file_move(&source_dir, &dest_dir);
                if success {
                    return output_file_name;
                }

                return String::new();
            }

            return server_file_name;
        }

        // 不存在
        if path.is_dir() {
            return file_name;
        }

        // 文件
        return path.file_name().unwrap().to_string_lossy().to_string();
    }

    /// 获取输出目录下的文件名
    fn get_path_file_name(&self, path: &Path) -> String {
        let entries = match fs::read_dir(path) {
            Ok(entries) => Some(entries),
            Err(err) => {
                println!("{} read dir {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), &path.to_string_lossy().to_string(), err);
                None
            }
        };

        if entries.is_none() {
            return String::new();
        }

        let mut files: Vec<String> = Vec::new();
        let mut directories: Vec<String> = Vec::new();

        let entries = entries.unwrap();
        for entry in entries {
            let entry = entry.unwrap();
            let file_path = entry.path();
            let relative_path = file_path.strip_prefix(path.to_string_lossy().to_string()).unwrap().to_string_lossy().to_string();
            if file_path.is_dir() {
                directories.push(relative_path);
            } else if file_path.is_file() {
                files.push(relative_path);
            }
        }

        // 只有1个目录, 则取该目录名称
        if directories.len() == 1 && files.is_empty() {
            return directories.get(0).unwrap().to_string();
        }

        // 只有一个文件, 取文件名称
        if directories.is_empty() && files.len() == 1 {
            return files.get(0).unwrap().to_string();
        }

        // 目录为空
        if directories.is_empty() && files.is_empty() {
            println!("{} upload dir {} is empty !", LOGGER_PREFIX.cyan().bold(), path.to_string_lossy().to_string().magenta().bold());
            return String::new();
        }

        // 其他情况, 获取路径最后一个名称
        return path.file_stem().unwrap().to_string_lossy().to_string();
    }

    /// 压缩目录
    fn compress_dir(&self, upload_dir: &str, server_file_name: &str, server_zip_file_name: &str) -> bool {
        // 判断上传目录下是文件还是目录, 如果是目录则压缩目录下的文件为 zip
        let path = PathBuf::from(upload_dir);
        if !path.is_dir() {
            return false;
        }

        let file_name = server_file_name.replace(".zip", "");

        // server_zip_file_name 目录
        let mut output_zip_path = PathBuf::from(upload_dir);
        output_zip_path.push(&server_zip_file_name);

        // file_name 目录
        let mut file_path = PathBuf::from(upload_dir);
        file_path.push(&file_name);

        let output_dir = output_zip_path.as_path().to_str().unwrap(); // 输出 .zip 文件路径
        let file_dir = file_path.as_path().to_str().unwrap();

        // 判断目录下是否有 file_name 的文件夹, 如果存在, 直接压缩该目录
        if file_path.exists() {
            return Utils::generate_zip(file_dir, output_dir);
        }

        // 不存在, 先在其创建 file_name 的文件夹, 并移动所有文件到其下面
        let success = match fs::create_dir(&file_path) {
            Ok(_) => true,
            Err(err) => {
                println!("{} create dir: {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), file_dir.magenta().bold(), err);
                false
            }
        };

        if !success {
            return false;
        }

        // 移动所有文件到 file_name 文件夹下面
        let dirs = match fs::read_dir(Path::new(upload_dir)) {
            Ok(dir) => Some(dir),
            Err(err) => {
                println!("{} read dir: {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), upload_dir.magenta().bold(), err);
                None
            }
        };

        if dirs.is_none() {
            return false;
        }

        let dirs = dirs.unwrap();
        let copy_options = dir::CopyOptions::new();
        for entry in dirs {
            let entry = entry.unwrap();
            let path = entry.path();
            let path_str = path.to_string_lossy().to_string();
            if path_str == file_dir {
                continue;
            }

            let mut from_paths = Vec::new();
            from_paths.push(path_str);
            if let Err(err) = move_items(&from_paths, file_dir, &copy_options) {
                println!("{} Failed to copy folder {}", LOGGER_PREFIX.cyan().bold(), err);
                return false;
            }
        }

        // 开始压缩
        let output_zip_file = String::from(output_dir) + ".zip";
        return Utils::generate_zip(file_dir, &output_zip_file);
    }

    /// 执行 shell 命令, linux 执行多条命令用 `\n` 分隔
    fn exec_command(&self, session: &Session, cmds: &Vec<String>) -> bool {
        println!("{} exec_command commands:\n {:#?}", LOGGER_PREFIX.cyan().bold(), cmds);

        if cmds.is_empty() {
            println!("{} commands is empty !", LOGGER_PREFIX.cyan().bold());
            return false;
        }

        // 创建 channel
        let channel = match session.channel_session() {
            Ok(channel) => Some(channel),
            Err(err) => {
                println!("{} use session create channel error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if channel.is_none() {
            return false;
        }

        let mut channel = channel.unwrap();
        let command = cmds.join(" \n ");
        println!("{} exec shell:\n {}", LOGGER_PREFIX.cyan().bold(), command.green().bold());

        let flag = match channel.exec(&command) {
            Ok(_) => {
                println!("{} {}", LOGGER_PREFIX.cyan().bold(), "exec command success".magenta().bold());
                true
            }
            Err(err) => {
                println!("{} exec command error: {:?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };

        let mut output = String::new();
        let flag = match channel.read_to_string(&mut output) {
            Ok(_) => flag,
            Err(err) => {
                println!("{} exec command error: {:?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };

        if !output.is_empty() {
            println!("{} command output:\n", LOGGER_PREFIX.cyan().bold());
            for line in output.lines() {
                println!("{}", line.green());
            }
        }

        let flag = match channel.send_eof() {
            Ok(_) => flag,
            Err(err) => {
                println!("{} close channel error: {:?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };

        let flag = match channel.wait_eof() {
            Ok(_) => flag,
            Err(err) => {
                println!("{} close channel error: {:?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };

        let flag = match channel.close() {
            Ok(_) => flag,
            Err(err) => {
                println!("{} close channel error: {:?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };

        // 关闭 channel
        return match channel.wait_close() {
            Ok(_) => flag,
            Err(err) => {
                println!("{} close channel error: {:?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };
    }
}
