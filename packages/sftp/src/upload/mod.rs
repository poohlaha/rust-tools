//! 文件上传, 压缩, 比较等

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use log::{error, info};
use ssh2::{Session, Sftp};
use uuid::Uuid;
use crate::sftp::SftpHandler;
use rayon::prelude::*;
use regex::Regex;
use handlers::error::Error;
use handlers::file::FileHandler;
use handlers::utils::Utils;
use crate::config::{Server, SftpUploadResult, Upload};

const UPLOAD_TEMP_DIR: &str = "__SFTP_TEMP_DIR__"; // 临时上传目录
pub struct SftpUpload;

#[derive(Debug, Default, Clone)]
struct SftpUploadDifferent {
    temp_path: String, // 临时文件全路径
    old_path: String,  // 找到的文件全路径
    relative_path: String, // 文件的相对路径
}

impl SftpUpload {

    pub fn exec<F>(server: Server, upload: Upload, log_func: F) -> Result<SftpUploadResult, String>
    where
        F: FnMut(&str)
    {
        if server.is_empty() {
            let msg = "exec upload failed, one of `host`、`port`、`username` and `password` server items is empty !";
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        if upload.is_empty() {
            let msg = "exec upload failed, one of `dir` and `server_dir` upload items is empty !";
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        let upload_dir_path = PathBuf::from(&upload.dir);
        if !upload_dir_path.exists() {
            let msg = format!("exec upload failed, upload dir: {} is not exists !", &upload.dir);
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 读取目录
        let (directories, files)= FileHandler::read_dir(&upload_dir_path)?;
        if directories.is_empty() && files.is_empty() {
            let msg = format!("exec upload failed, upload dir: {} is empty !", &upload.dir);
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 获取上传文件名
        let file_name = Self::get_upload_file_name(&upload, directories.clone(), files.clone())?;
        if file_name.is_empty() {
            let msg = "exec upload failed, can not get filename !";
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        let log_func = Arc::new(Mutex::new(log_func));

        // 输出日志
        SftpHandler::log_info(&format!("get upload filename: {}", file_name), log_func.clone());

        // 文件名路径
        let file_path = PathBuf::from(&upload.dir).join(&file_name);

        // 压缩目录
        let zip_file_path = Self::compress_upload_dir(&upload, &file_path, directories.clone(), files.clone())?;
        let zip_file_path = Self::rename_file_upload_path(&zip_file_path)?; // 临时文件目录

        // 连接服务器
        let session = SftpHandler::connect(&server)?;
        let sftp = session.sftp().map_err(|err| {
            let msg = format!("exec upload error: {:#?}", err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        // 文件上传和发布
        let result = Self::upload_and_publish(&session, &sftp, &upload, &zip_file_path, &file_name, log_func.clone())?;
        Ok(result)
    }

    /// 获取上传文件名
    fn get_upload_file_name(upload: &Upload, directories: Vec<String>, files: Vec<String>) -> Result<String, String> {
        if let Some(server_file_name) = &upload.server_file_name {
            return Ok(server_file_name.to_string());
        }

        let path = Path::new(&upload.dir);

        // 1. 只有1个目录, 则取该目录名称
        if directories.len() == 1 && files.is_empty() {
            return Ok(directories.get(0).unwrap_or(&String::new()).to_string());
        }

        // 2. 只有一个文件, 取文件名称
        if directories.is_empty() && files.len() == 1 {
            return Ok(files.get(0).unwrap_or(&String::new()).to_string());
        }

        // 3. 目录为空
        if directories.is_empty() && files.is_empty() {
            let msg = "upload failed, can not get filename";
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 4. 其他情况, 取路径最后一个名称
        Ok(path.file_stem().unwrap_or(OsStr::new("")).to_string_lossy().to_string())
    }

    /// 压缩文件
    fn compress_upload_dir(upload: &Upload, file_path: &PathBuf, directories: Vec<String>, files: Vec<String>) -> Result<String, String> {
        // 文件名路径
        let file_path_str = file_path.to_string_lossy().to_string();

        // zip 包路径
        let mut zip_file_path = file_path.clone();
        zip_file_path.set_extension("zip");
        let zip_file_path_str = zip_file_path.to_string_lossy().to_string();
        // 1. 判断目录下有没有待上传文件.zip 包
        if directories.len() == 0 && files.len() == 1 && zip_file_path.exists() {
            return Ok(zip_file_path.to_string_lossy().to_string());
        }

        // 2. 当目录下存在且只有一个 `文件名` 的目录
        if directories.len() == 1 && files.len() == 0 && file_path.exists() {
            return Self::generate_zip(&file_path_str, &zip_file_path_str);
        }

        // 3. 其他情况: 创建目录, 移动目录到新目录, 然后压缩
        // 创建随机数目录, 防止存在和文件名相同的目录
        let file_random_name = Uuid::new_v4().to_string();
        let file_random_path = PathBuf::from(&upload.dir).join(file_random_name);
        let file_random_path_str = file_random_path.to_string_lossy().to_string();

        FileHandler::create_dirs(&file_random_path_str)?;

        let move_directories: Vec<String> = directories.clone().iter().map(|dir| Path::new(&upload.dir).join(dir).to_string_lossy().to_string()).collect();
        let move_files: Vec<String> = files.clone().iter().map(|dir| Path::new(&upload.dir).join(dir).to_string_lossy().to_string()).collect();

        // 移动目录到随机数目录
        match FileHandler::move_dirs(move_directories, &file_random_path_str) {
            Ok(_) => {}
            Err(error) => {
                let msg = format!("compress upload dir error: {}", error);
                error!("{}", &msg);
                FileHandler::delete_dirs(vec![file_random_path_str.clone()]).unwrap_or(());
                return Err(Error::convert_string(&msg));
            }
        }

        // 移动文件到随机数目录
        match FileHandler::move_files(move_files, &file_random_path_str) {
            Ok(_) => {}
            Err(error) => {
                let msg = format!("compress upload dir error: {}", error);
                error!("{}", &msg);
                FileHandler::delete_dirs(vec![file_random_path_str.clone()]).unwrap_or(());
                return Err(Error::convert_string(&msg));
            }
        }

        // 更改随机数目录为文件目录
        FileHandler::rename(&file_random_path_str, &file_path_str)?;

        // 压缩目录
        return Self::generate_zip(&file_path_str, &zip_file_path_str);
    }

    /// 生成 zip
    fn generate_zip(file_path: &str, zip_file_path: &str) -> Result<String, String> {
        let success = Utils::generate_zip(file_path, zip_file_path)?;
        if !success {
            let msg = format!("upload failed, generate zip: {:#?} failed !", zip_file_path);
            error!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 成功后删除原来目录
        FileHandler::delete_dirs(vec![file_path.to_string()])?;
        return Ok(zip_file_path.to_string());
    }

    /// 文件上传
    fn upload_and_publish<F>(session: &Session, sftp: &Sftp, upload: &Upload, zip_file_path: &str, file_name: &str, log_func: Arc<Mutex<F>>) -> Result<SftpUploadResult, String>
    where
        F: FnMut(&str)
    {
        // 获取远程临时存放目录
        let server_dir = &upload.server_dir.trim();
        let mut server_temp_path = PathBuf::from(server_dir);
        if let Some(parent) = server_temp_path.parent() {
            server_temp_path = parent.join(&UPLOAD_TEMP_DIR)
        } else {
            server_temp_path = server_temp_path.join(&UPLOAD_TEMP_DIR);
        }

        let server_temp_path_str = server_temp_path.to_string_lossy().to_string(); // 远程临时存放目录
        let zip_file_name = Path::new(zip_file_path).file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();

        // 1. 上传
        SftpHandler::log_info(&format!("begin to uploading file {} and set file permission ...", zip_file_path), log_func.clone());

        SftpHandler::upload(sftp, zip_file_path, &server_temp_path_str, &zip_file_name)?;

        SftpHandler::log_info(&format!("uploading file {} and set file permission success !", zip_file_path), log_func.clone());

        // 2. 解压
        let unzip_dir = Path::new(&server_temp_path_str).join(file_name);
        let unzip_dir_str = unzip_dir.to_string_lossy().to_string();

        let server_file_path = PathBuf::from(&server_temp_path_str).join(&zip_file_name);
        match Self::uncompress_zip(session, sftp, &server_temp_path_str, &zip_file_name, &unzip_dir_str, log_func.clone()) {
            Ok(_) => {}
            Err(err) => {
                let msg = format!("uncompress zip: {:?} error: {:#?} !", server_file_path, err);
                error!("{}", msg);
                Self::end(sftp, session, &server_file_path, &unzip_dir_str, zip_file_path, log_func.clone());
                return Err(Error::convert_string(&msg));
            }
        };

        // 3. 比较发布
        let server_file_dir = Path::new(&upload.server_dir).join(file_name);

        // 获取发布命令
        let result = match Self::touch_publish_commands(sftp, &upload, &server_file_dir.to_string_lossy().to_string(), &unzip_dir_str, log_func.clone()) {
            Ok(result) => {
                result
            }
            Err(err) => {
                let msg = format!("publish {} error: {}", file_name, err);
                error!("{}", &msg);
                Self::end(sftp, session, &server_file_path, &unzip_dir_str, zip_file_path, log_func.clone());
                return Err(Error::convert_string(&msg));
            }
        };

        // 输出日志
        SftpHandler::log_info(&format!("exec commands:\n {:#?}", result.exec_commands), log_func.clone());

        // 执行发布命令
        if result.exec_commands.is_empty() {
            // 输出日志
            SftpHandler::log_info("no commands need to exec !", log_func.clone());
            Self::end(sftp, session, &server_file_path, &unzip_dir_str, zip_file_path, log_func.clone());
            return Ok(result)
        }

        match Self::exec_command(session, result.exec_commands.clone(), log_func.clone()) {
            Ok(_) => {
                Self::end(sftp, session, &server_file_path, &unzip_dir_str, zip_file_path, log_func.clone());
            }
            Err(err) => {
                // 输出日志
                let msg = format!("publish {} error: {}", file_name, err);
                SftpHandler::log_error("no commands need to exec !", log_func.clone());
                Self::end(sftp, session, &server_file_path, &unzip_dir_str, zip_file_path, log_func.clone());
                return Err(Error::convert_string(&msg));
            }
        }

        Ok(result)
    }

    /// 结束
    fn end<F>(sftp: &Sftp, session: &Session, server_file_path: &PathBuf, unzip_dir_str: &str, zip_file_path: &str, log_func: Arc<Mutex<F>>)
    where
        F: FnMut(&str)
    {
        SftpHandler::log_info(&format!("upload end, begin to delete local and server zip file: {:?} 、 unzip dir: {}", server_file_path, unzip_dir_str), log_func.clone());

        let _ = sftp.unlink(&server_file_path).map_err(|err| {
            let msg = format!("delete file `{:?}` error: {:#?}", server_file_path, err);
            SftpHandler::log_error(&msg, log_func.clone());
            Error::convert_string(&msg)
        });

        let _ = Self::exec_command(session, vec![format!("rm -rf {}", unzip_dir_str)], log_func.clone());

        // 删除本地压缩包
        let _ = FileHandler::delete_file(zip_file_path);
        SftpHandler::log_info(&format!("upload end, delete local and server zip file: {:?} 、 unzip dir: {} success !", server_file_path, unzip_dir_str), log_func.clone());
    }

    /// 重命令上传目录，添加时间戳
    fn rename_file_upload_path(zip_file_path: &str) -> Result<String, String> {
        // 获取临时文件名
        let data_suffix: String = chrono::Local::now().format("%Y%m%d%H%M%S").to_string(); // 生成时间后缀

        let mut temp_file_path = PathBuf::from(&zip_file_path);
        let temp_file_name = temp_file_path.file_stem().unwrap().to_str().unwrap_or("").to_string() + "-" + data_suffix.as_str();
        temp_file_path.set_file_name(&temp_file_name);
        temp_file_path.set_extension("zip");

        let temp_file_str= temp_file_path.to_string_lossy().to_string();
        info!("get upload temp filename: {}", temp_file_name);

        // 重命名文件
        FileHandler::rename(&zip_file_path, &temp_file_str)?;
        Ok(temp_file_str)
    }

    /// 远程解压 zip 包
    fn uncompress_zip<F>(session: &Session, sftp: &Sftp, upload_temp_dir: &str, zip_file_name: &str, unzip_dir_str: &str, log_func: Arc<Mutex<F>>) -> Result<(), String>
    where
        F: FnMut(&str)
    {
        let zip_file_path = Path::new(&upload_temp_dir).join(zip_file_name);
        let zip_file_path_str = zip_file_path.to_string_lossy().to_string();

        if !sftp.stat(&zip_file_path).is_ok() {
            let msg = format!("uncompress server zip file failed, zip path: {:?} not exists !", zip_file_path);
            error!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        let commands: Vec<String> = vec![
          format!("cd {}", upload_temp_dir),
          format!("rm -rf {}", unzip_dir_str),
          format!("unzip {} -d {}", &zip_file_path_str, upload_temp_dir), // unzip 到指定目录 -o 为强制覆盖
        ];

        SftpHandler::log_info(&format!("unzip: {} ... ", zip_file_path_str), log_func.clone());
        Self::exec_command(session, commands.clone(), log_func.clone())?;
        SftpHandler::log_info(&format!("unzip: {} success !", zip_file_path_str), log_func.clone());
        Ok(())
    }

    /// 发布
    /// 判断是不是增量发布
    /// 非增量发布, 需要比较文件夹内的文件是否一致, 如果不一致则替换
    fn touch_publish_commands<F>(sftp: &Sftp, upload: &Upload, file_dir: &str, temp_file_dir: &str, log_func: Arc<Mutex<F>>) -> Result<SftpUploadResult, String>
    where
        F: FnMut(&str)
    {
        // 判断两个目录是否存在
        let temp_file_path = Path::new(temp_file_dir);
        let file_file_path = Path::new(file_dir);

        // 临时目录不存在
        if !sftp.stat(temp_file_path).is_ok() {
            let msg = format!("publish dir failed, temp dir `{:?}` is not exists !", temp_file_path);
            error!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 读取临时目录下的文件
        let mut temp_files: Vec<String> = Vec::new();
        Self::read_files(sftp, temp_file_dir, &mut temp_files);

        if temp_files.is_empty() {
            let msg = format!("publish dir failed, temp dir `{:?}` is empty !", temp_file_path);
            error!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 全量发布
        let get_full_publish_cmds = || {
            SftpHandler::log_info(&format!("use full publish, file count: {} ...", temp_files.len()), log_func.clone());
            let mut result = SftpUploadResult::default();

            let mut cmds: Vec<String> = Vec::new();
            cmds.push(format!("rm -rf {}", file_dir)); // 删除原来的文件目录
            cmds.push(format!("mv {} {}", temp_file_dir, &upload.server_dir)); // 移动临时目录到原来的文件目录

            result.file_count = temp_files.len() as u64;
            result.need_increment = false;
            result.exec_commands = cmds;
            return result
        };

        // 1. 目标目录不存在, 则直接采用全量发布(全量)
        if !sftp.stat(file_file_path).is_ok() {
            return Ok(get_full_publish_cmds())
        }

        // 2. 当 need_increment 为 false 时, 使用全量发布(全量)
        if !upload.need_increment {
            return Ok(get_full_publish_cmds())
        }

        // 读取目录文件列表
        let mut files: Vec<String> = Vec::new();
        Self::read_files(sftp, file_dir, &mut files);

        // 3. 没有文件, 则取全量发布(全量)
        if files.len() == 0 {
            return Ok(get_full_publish_cmds())
        }

        // 4. 当 need_increment 为 true 时, 使用增量发布(增量)
        let mut commands: Vec<String> = Vec::new();
        let mut result = SftpUploadResult::default();

        // 用临时目录和比较原来目录进行比较, 获取不同的文件
        let differences = Self::get_compare_file(sftp, &files, &temp_files, file_dir, temp_file_dir, log_func.clone());
        SftpHandler::log_info(&format!("difference file count: {}", differences.len()), log_func.clone());

        result.file_count = differences.len() as u64; // 设置发布文件个数

        let remove_cmds = Self::remove_no_used_files_in_dir(&files, &temp_files, file_dir, temp_file_dir, log_func.clone());
        result.delete_file_count = remove_cmds.len() as u64; // 设置删除文件个数
        SftpHandler::log_info(&format!("remove cmds: \n {:#?}", remove_cmds), log_func.clone());

        if differences.is_empty() {
            SftpHandler::log_info("no difference files, publish skip !", log_func.clone());
            // 移除没用的文件
            commands.extend(remove_cmds);
        } else {
            let cmds = Self::get_increment_files_commands(sftp, &differences, file_dir, log_func.clone());
            SftpHandler::log_info(&format!("increment cmds: \n {:#?}", cmds), log_func.clone());
            commands.extend(cmds);
            commands.extend(remove_cmds)
        }

        result.exec_commands = commands;
        Ok(result)
    }

    /// 获取两个目录的比较文件, 此处使用并行任务并没有快多少
    fn get_compare_file<F>(sftp: &Sftp, files: &Vec<String>, temp_files: &Vec<String>, file_dir: &str, temp_file_dir: &str, log_func: Arc<Mutex<F>>) -> Vec<SftpUploadDifferent>
    where
        F: FnMut(&str)
    {
        let mut differences: Vec<SftpUploadDifferent> = Vec::new();

        // 根据新文件来比较旧文件
        temp_files.iter().for_each(|temp_file| {
            let temp_file_name = Path::new(temp_file).file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
            let temp_file_relative_path = Path::new(&temp_file).strip_prefix(temp_file_dir).unwrap_or(&Path::new("")).to_string_lossy().to_string();

            // 查找文件是否需要替换
            let find_file = files.par_iter().find_first(|f| Self::compare_two_file_same(f, temp_file, file_dir, temp_file_dir));

            if let Some(find_file) = find_file {
                let file_name = Path::new(find_file).file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
                let temp_file_name = Path::new(temp_file).file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
                let file_name_extension = Path::new(find_file).extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
                let temp_file_name_extension = Path::new(temp_file).extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");

                // 判断文件名是否一致
                let has_same = Self::math_filename_hash(&file_name, &temp_file_name, file_name_extension, temp_file_name_extension);
                // 带 hash 值, 判断文件名是不是一致
                if has_same {
                    if &file_name == &temp_file_name {
                        SftpHandler::log_info(&format!("filename {} same, compare file hash !", &file_name), log_func.clone());
                        if !Self::compare_two_file_hash(sftp, find_file, temp_file, log_func.clone()) {
                            differences.push(SftpUploadDifferent {
                                temp_path: temp_file.to_string(),
                                old_path: find_file.to_string(),
                                relative_path: temp_file_relative_path.clone(),
                            });
                        }
                    } else {
                        differences.push(SftpUploadDifferent {
                            temp_path: temp_file.to_string(),
                            old_path: find_file.to_string(),
                            relative_path: temp_file_relative_path.clone(),
                        });
                    }
                } else {
                    // 不带 hash, 判断文件名和 hash 是否一致
                    if &file_name == &temp_file_name {
                        SftpHandler::log_info(&format!("filename {} same but it has no hash code, compare file hash !", &file_name), log_func.clone());
                        if !Self::compare_two_file_hash(sftp, find_file, temp_file, log_func.clone()) {
                            differences.push(SftpUploadDifferent {
                                temp_path: temp_file.to_string(),
                                old_path: find_file.to_string(),
                                relative_path: temp_file_relative_path.clone(),
                            });
                        }
                    }
                }
            } else {
                // 没有找到, 需要拷贝
                if find_file.is_none() {
                    SftpHandler::log_info(&format!("filename {} is new, it will be added !", &temp_file_name), log_func.clone());
                    differences.push(SftpUploadDifferent {
                        temp_path: temp_file.to_string(),
                        old_path: String::new(),
                        relative_path: temp_file_relative_path.clone(),
                    });
                }
            }
        });

        return differences;
    }

    /// 比较两个文件是否相同, 包含文件名中带有 hash 值的文件
    fn compare_two_file_same(file: &str, temp_file: &str, file_dir: &str, temp_file_dir: &str) -> bool {
        // 1. 比较两文件的相对路径是否一致
        let file_relative_dir = file.replace(file_dir, "");
        let temp_file_relative_dir = temp_file.replace(temp_file_dir, "");
        if &file_relative_dir == &temp_file_relative_dir {
            return true;
        }

        // 2. 比较去掉文件名后的相对路径是否一致, 比如 /usr/local/test.js 和 /usr/local/sample.js, parent 为 /usr/local/
        let file_relative_parent_path = Path::new(&file_relative_dir).parent().unwrap_or(&Path::new(""));
        let temp_file_relative_parent_path =  Path::new(&temp_file_relative_dir).parent().unwrap_or(&Path::new(""));

        let file_relative_parent_str = file_relative_parent_path.to_string_lossy().to_string();
        let temp_file_relative_parent_str = temp_file_relative_parent_path.to_string_lossy().to_string();
        if &file_relative_parent_str != &temp_file_relative_parent_str {
            return false;
        }

        // 3. 比较文件名是否一致(不带 hash 值)
        let file_name = Path::new(file).file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
        let temp_file_name = Path::new(temp_file).file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
        let file_name_extension = Path::new(file).extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
        let temp_file_name_extension = Path::new(temp_file).extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");

        if &file_name == &temp_file_name {
           return true
        }

        // 4. 比较文件名是否一致(带 hash 值)
        // 4.1 判断文件是否以 hash 值开头, 以 hash 开头则需要替换
        let success = Self::math_filename_start_with_hash(&file_name, &temp_file_name, file_name_extension, temp_file_name_extension);
        if success {
            return false;
        }

        // 4.2 判断文件去除 hash 值后是否一致, 如果去除后一致, 则返回
        return Self::judge_filename_hash(&file_name, &temp_file_name);
    }

    /// 比较两个文件的 hash 值是否一致
    fn compare_two_file_hash<F>(sftp: &Sftp, file: &str, temp_file: &str, log_func: Arc<Mutex<F>>) -> bool
    where
        F: FnMut(&str)
    {
        let file_hash = match SftpHandler::get_file_hash(sftp, file) {
            Ok(hash) => hash,
            Err(err) => {
                SftpHandler::log_error(&format!("get file `{}` hash error: {}", file, err), log_func.clone());
                String::new()
            }
        };

        if file_hash.is_empty() {
            return false;
        }

        let temp_file_hash = match SftpHandler::get_file_hash(sftp, temp_file) {
            Ok(hash) => hash,
            Err(err) => {
                SftpHandler::log_error(&format!("get file `{}` hash error: {}", temp_file, err), log_func.clone());
                String::new()
            }
        };

        if temp_file_hash.is_empty() {
            return false;
        }

        return file_hash == temp_file_hash;
    }

    /// 判断文件是否以 hash 值开头
    fn math_filename_start_with_hash(file1_name: &str, file2_name: &str, file1_extension: &str, file2_extension: &str) -> bool {
        let hash_pattern = Regex::new(r"^[0-9a-fA-F.]+").unwrap();
        let match_name1 = hash_pattern.replace(&file1_name, "");
        let match_name2 = hash_pattern.replace(&file2_name, "");
        return match_name1 == file1_extension && match_name2 == file2_extension;
    }

    /// 检验文件名中去除包含的hash值是否一样
    pub fn judge_filename_hash(file1_name: &str, file2_name: &str) -> bool {
        let hash_pattern = Regex::new(r"\.[0-9a-fA-F.]+").unwrap();
        if !hash_pattern.is_match(file1_name) || !hash_pattern.is_match(file2_name) {
            return false;
        }

        let new_filename1 = hash_pattern.replace(file1_name, ".").to_string();
        let new_filename2 = hash_pattern.replace(file2_name, ".").to_string();
        return new_filename1 == new_filename2;
    }

    /// 判断文件名中是否带有hash
    pub fn math_filename_hash(file1_name: &str, file2_name: &str, file1_extension: &str, file2_extension: &str) -> bool {
        let success = Self::math_filename_start_with_hash(file1_name, file2_name, file1_extension, file2_extension);
        if success {
            return true;
        }

        let hash_pattern = Regex::new(r"\.[0-9a-fA-F.]+").unwrap();
        if !hash_pattern.is_match(file1_name) || !hash_pattern.is_match(file2_name) {
            return false;
        }

        return true;
    }

    /// 移除不用的文件
    fn remove_no_used_files_in_dir<F>(files: &Vec<String>, temp_files: &Vec<String>, file_dir: &str, temp_file_dir: &str, log_func: Arc<Mutex<F>>) -> Vec<String>
    where
        F: FnMut(&str)
    {
        let commands: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

        files.iter().for_each(|file| {
            // 比较文件
            let file_relative_path = Path::new(&file).strip_prefix(file_dir).unwrap_or(&Path::new("")).to_string_lossy().to_string();
            let find_file = temp_files.par_iter().find_first(|f| {
                let temp_file_relative_path = Path::new(f).strip_prefix(temp_file_dir).unwrap_or(&Path::new("")).to_string_lossy().to_string();
                return &file_relative_path == &temp_file_relative_path
            });

            // 未找到文件, 则需要删除
            if find_file.is_none() {
                SftpHandler::log_info(&format!("file {} is not used, will be deleted !", file), log_func.clone());
                let mut commands = commands.lock().unwrap();
                commands.push(format!("rm -rf {}", file));
            }
        });

        let commands = commands.lock().unwrap();
        let commands = commands.clone();
        if commands.is_empty() {
            SftpHandler::log_info("no used file to be delete !", log_func.clone());
        }

        return commands;
    }

    /// 获取增量发布的命令
    fn get_increment_files_commands<F>(sftp: &Sftp, differences: &Vec<SftpUploadDifferent>, file_dir: &str, log_func: Arc<Mutex<F>>) -> Vec<String>
    where
        F: FnMut(&str)
    {
        let mut commands: Vec<String> = Vec::new();

        differences.iter().for_each(|d| {
            let temp_path = Path::new(&d.temp_path);

            // 判断路径是不是存在
            if sftp.stat(temp_path).is_ok() {
                // 获取旧文件
                if !d.relative_path.is_empty() {
                    let file_path = Path::new(file_dir).join(&d.relative_path);
                    commands.push(format!("rm -rf {}", d.old_path)); // 1. 删除旧的文件
                    commands.push(format!("cp {} {}", d.temp_path, &file_path.as_path().to_string_lossy().to_string()));
                }
            } else {
                SftpHandler::log_info(&format!("file path {} not exists !", &d.temp_path), log_func.clone())
            }
        });

        return commands;
    }

    /// 读取目录中的文件
    fn read_files(sftp: &Sftp, dir: &str, files: &mut Vec<String>){
        let entries = match sftp.readdir(Path::new(dir)) {
            Ok(entries) => {
                entries
            }
            Err(err) => {
                info!("read dir `{}` files error: {:#?}", dir, err);
                Vec::new()
            }
        };

        for (path, file_stat) in entries.iter() {
            let path = path.to_string_lossy().to_string();
            if file_stat.is_dir() {
                Self::read_files(sftp, &path, files);
            } else {
                files.push(path);
            }
        }
    }

    fn exec_command<F>(session: &Session, cmds: Vec<String>, log_func: Arc<Mutex<F>>) -> Result<(), String>
    where
        F: FnMut(&str)
    {
        let mut channel = SftpHandler::create_channel(session)?;
        let command = cmds.join(" \n ");
        SftpHandler::log_info(&format!("exec server command:\n {}", command), log_func.clone());

        channel.exec(&command).map_err(|err| Error::Error(err.to_string()).to_string())?;
        let (content, error) = SftpHandler::get_channel_output(&mut channel)?;
        if !error.is_empty() {
            let msg = format!("exec server commands error: {}", &error);
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 打印日志
        SftpHandler::log_info("command output: \n", log_func.clone());
        for line in content.lines() {
            info!("{}", line);
        }

        SftpHandler::close_channel(&mut channel)?;
        Ok(())
    }
}