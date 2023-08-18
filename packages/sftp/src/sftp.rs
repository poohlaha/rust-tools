//!  执行 linux 命令, 文件上传等操作

use std::ffi::OsStr;
use std::{fs};
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;
use colored::*;
use ssh2::{Session};
use fs_extra::move_items;
use fs_extra::dir;
use chrono::Local;
use indicatif::{ProgressBar, ProgressStyle};
use crate::utils::Utils;
use crate::config::{Server, Upload};
use crate::LOGGER_PREFIX;

pub struct Sftp {
    server: Server,
    upload: Upload
}

impl Sftp {
    pub fn new(mut server: Server, mut upload: Upload) -> Self {
        if server.is_empty() {
            server = Server::default()
        }

        if upload.is_empty() {
            upload = Upload::default()
        }
        return Self {
            server,
            upload
        }
    }

    /// 连接服务器, 处理逻辑
    pub fn send(&self) -> bool {
        let server = &self.server;
        if server.is_empty() {
            println!("{} one of `host`、`port`、`username` and `password` items is empty !", LOGGER_PREFIX.cyan().bold());
            return false;
        }

        let cmds = &self.upload.cmds;

        // 处理输出目录, 压缩成 .zip 文件
        let (server_file_name, server_zip_file_name) = self.hand_output_dir();
        if cmds.is_empty() && (server_file_name.is_empty() || server_zip_file_name.is_empty()) {
            return false;
        }

        // 拼接服务器地址
        let address = format!("{}:{}", &server.host, server.port);
        let tcp = match TcpStream::connect(&address) {
            Ok(tcp) => {
                Some(tcp)
            },
            Err(err) => {
                println!("{} connect to {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), address.clone().red().bold(), err);
                None
            }
        };

        // 创建 session
        let session = match Session::new() {
            Ok(session) => Some(session),
            Err(err) => {
                println!("{} create session error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if session.is_none() {
            return false;
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
                return false;
            }
        }

        // 认证用户名密码是否正确
        if !session.authenticated() {
            println!("{} authentication server: {} failed !", LOGGER_PREFIX.cyan().bold(), address.clone().red().bold());
        }

        println!("{} connect {} success !", LOGGER_PREFIX.cyan().bold(), address.clone().magenta().bold());

        // 处理文件上传
        let mut success = false;
        if !server_file_name.is_empty() && !server_zip_file_name.is_empty() {
            success = self.upload(&session, &server_file_name, &server_zip_file_name, &self.upload);
        } else {
            // 直接执行命令
            success = self.exec_command(&session, cmds);
        }

        // 关闭 session
        let flag = match session.disconnect(None, "Bye bye !", None) {
            Ok(_) => success,
            Err(err) => {
                println!("{} close session error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };


        if self.upload.is_empty() {
            return flag;
        }

        // 判断目录是否存在
        if !Path::new(&self.upload.dir).exists() {
            return flag;
        }

        // 删除本地目录
        println!("{} begin to {} upload dir: {}", LOGGER_PREFIX.cyan().bold(), "delete".red().bold(), &self.upload.dir.green().bold());
        let flag = match fs::remove_dir_all(&self.upload.dir) {
            Ok(_) => flag,
            Err(err) => {
                println!("{} delete upload dir {} failed: {:#?}", LOGGER_PREFIX.cyan().bold(), &self.upload.dir.red().bold(), err);
                false
            }
        };

        if flag {
            println!("{} {} upload dir {} success !", LOGGER_PREFIX.cyan().bold(), "delete".red().bold(), &self.upload.dir.green().bold());
        }

        return flag;
    }

    /// 文件上传
    fn upload(&self, session: &Session, server_file_name: &str, server_zip_file_name: &str, upload: &Upload) -> bool {
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
            return false;
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
            return false;
        }

        // 创建 sftp
        let sftp = match session.sftp() {
            Ok(sftp) => {
                Some(sftp)
            },
            Err(err) => {
                println!("{} use session sftp error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if sftp.is_none() {
            return false;
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
            return false;
        }

        let mut remote_file = remote_file.unwrap();

        println!("{} uploading file: {}", LOGGER_PREFIX.cyan().bold(), local_file_path.clone().green().bold());

        // 进行 process bar
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_style(
            ProgressStyle::with_template("{spinner:.blue} {msg}")
                .unwrap()
                .tick_strings(&[
                    "▹▹▹▹▹",
                    "▸▹▹▹▹",
                    "▹▸▹▹▹",
                    "▹▹▸▹▹",
                    "▹▹▹▸▹",
                    "▹▹▹▹▸",
                    "▪▪▪▪▪",
                ]),
        );

        pb.set_message("Uploading...");

        // 写入远程文件
        let success = match remote_file.write_all(&buffer) {
            Ok(_) => {
                Some(1)
            },
            Err(err) => {
                println!("{} write local file to remote file error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                None
            }
        };

        if success.is_none() {
            println!("{} upload file: {} failed !", LOGGER_PREFIX.cyan().bold(), local_file_path.clone().red().bold());
            return false;
        }

        pb.finish_with_message("Upload Success !");

        println!("{} upload file success, file path: {}", LOGGER_PREFIX.cyan().bold(), server_file.clone().green().bold());

        // 解压 zip 包, 删除原来目录，用新的目录覆盖
        return self.uncompress(session, &sftp, server_file_name, server_zip_file_name, upload);
    }

    /// 获取远程解压服务器下的文件夹
    fn get_remote_dir(&self, sftp: &ssh2::Sftp, unzip_dir: &str) -> Vec<String> {
        let entries = match sftp.readdir(Path::new(unzip_dir)) {
            Ok(entries) => Some(entries),
            Err(err) => {
                println!("{} session sftp read dir {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), unzip_dir.magenta().bold(), err);
                None
            }
        };

        let mut directories: Vec<String> = Vec::new();
        if entries.is_none() {
            return directories;
        }

        let entries = entries.unwrap();
        for (path, file_stat) in entries.iter() {
            if file_stat.is_dir() {
                directories.push(path.to_string_lossy().to_string());
            }
        }

        return directories
    }

    /// 解压 zip 包, 删除原来目录，用新的目录覆盖
    fn uncompress(&self, session: &Session, sftp: &ssh2::Sftp, server_file_name: &str, server_zip_file_name: &str, upload: &Upload) -> bool {
        let server_dir = &upload.server_dir;
        println!("{} begin to unzip {} ...", LOGGER_PREFIX.cyan().bold(), format!("{}{}", server_dir, server_zip_file_name).green().bold());

        // 解压
        let server_zip_file_stem: &str = Path::new(server_zip_file_name).file_stem().unwrap().to_str().unwrap_or(""); // 文件前缀
        let unzip_dir = Utils::get_path(server_dir, &[&server_zip_file_stem]);
        let server_project_dir_stem: &str = Path::new(server_file_name).file_stem().unwrap().to_str().unwrap_or(""); // 文件前缀
        let server_project_dir = Utils::get_path(server_dir, &[&server_project_dir_stem]); // 原来项目目录

        let commands: Vec<String> = vec![
            format!("cd {}", server_dir), // cd 到服务器目录
            format!("unzip {} -d {}", server_zip_file_name, unzip_dir)  // unzip 到指定目录 -o 为强制覆盖
        ];

        let success = self.exec_command(session, &commands);
        if !success {
            println!("{} unzip {} failed !", LOGGER_PREFIX.cyan().bold(), format!("{}{}", server_dir, server_zip_file_name).red().bold());
            // 删除解压目录和压缩包
            let commands: Vec<String> = vec![
                format!("cd {}", server_dir), // cd 到服务器目录
                format!("rm -rf {}", unzip_dir), // 删除已解压目录
                format!("rm -rf {}", server_zip_file_name)  // 删除压缩包
            ];
            self.exec_command(session, &commands);
            return false;
        }

        println!("{} unzip {} success !", LOGGER_PREFIX.cyan().bold(), format!("{}{}", server_dir, server_zip_file_name).green().bold());

        println!("{} begin to {} version ...", LOGGER_PREFIX.cyan().bold(), "publishing".green().bold());
        // 删除原来的项目, 更改解压后的目录为原来目录
        let mut commands: Vec<String> = vec![
            format!("cd {}", server_dir), // cd 到服务器目录
            format!("rm -rf {}", server_project_dir), // 删除原来的目录
        ];

        // 判断 unzip 解压目录下是否只有一个目录
        let directories = self.get_remote_dir(sftp, &unzip_dir);
        // 只有一个, 则移动该目录到最外层
        if directories.len() == 1 {
            let dir_path: String = directories.get(0).unwrap().to_string();
            commands.push( format!("mv {} {}", dir_path, &server_dir)); // 移动到最外层
        } else { // 移动文件成原来目录
            commands.push(format!("mv {} {}", &unzip_dir, &server_dir)); // 移动文件成原来目录
        }

        commands.push(format!("rm -rf {}", unzip_dir)); // 删除压缩目录
        commands.push(format!("rm -rf {}", server_zip_file_name)); // 删除压缩包

        // 合并服务端命令
        let cmds: Vec<String> = upload.cmds.iter().map(|cmd|cmd.to_string()).collect();
        commands.extend(cmds);
        let success = self.exec_command(session, &commands);
        if !success {
            println!("{} publish version failed ...", LOGGER_PREFIX.red().bold());
        }

        println!("{} {} ...", LOGGER_PREFIX.cyan().bold(), "publish version success".green().bold());
        return success
    }

    /// 处理输出目录, 压缩成 .zip 文件
    fn hand_output_dir(&self) -> (String, String) {
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
        let mut server_zip_file_name = String::new();
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
            let success = Utils::file_move(&file_path, &output_file_path);
            if !success {
                return (String::new(), String::new());
            }

            return (server_file_name.clone(), server_zip_file_name.clone());
        }

        // 不是.zip 文件, 则压缩成 zip 文件
        println!("{} begin to compress dir {} to zip ...", LOGGER_PREFIX.cyan().bold(), upload_dir.magenta().bold());
        let success = self.compress_dir(upload_dir, &server_zip_file_name);
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
                println!("source_dir: {}", source_dir);
                println!("dest_dir: {}", dest_dir);
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
    fn compress_dir(&self, upload_dir: &str, server_file_name: &str) -> bool {
        // 判断上传目录下是文件还是目录, 如果是目录则压缩目录下的文件为 zip
        let mut path = PathBuf::from(upload_dir);
        if !path.is_dir() {
            return false;
        }

        // 判断目录下是否有 server_file_name 的文件夹
        let mut file_path = PathBuf::from(upload_dir);
        file_path.push(server_file_name);

        path.push(server_file_name);
        let output_dir = path.as_path().to_str().unwrap(); // 输出 .zip 文件路径
        let file_dir = file_path.as_path().to_str().unwrap();

        // 如果存在, 直接压缩该目录
        if file_path.exists() {
            return Utils::generate_zip(file_dir, output_dir);
        }

        // 不存在, 先在其创建 server_file_name 的文件夹, 并移动所有文件到其下面
        let success = match fs::create_dir(&file_path){
            Ok(_) => true,
            Err(err) => {
                println!("{} create dir: {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), file_dir.magenta().bold(), err);
                false
            }
        };

        if !success {
            return false;
        }

        // 移动所有文件到 server_file_name 文件夹下面
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
            },
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
        return match channel.wait_close(){
            Ok(_) => flag,
            Err(err) => {
                println!("{} close channel error: {:?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };
    }

}