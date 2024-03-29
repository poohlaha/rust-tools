//! 远程文件对比，并运行读取日志

use crate::config::{Server, ValidateCopy};
use crate::sftp::SftpHandler;
use handlers::error::Error;
use log::{error, info};
use ssh2::{Session, Sftp};
use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct SftpRunnableHandler;

impl SftpRunnableHandler {
    pub fn exec<F>(server: Server, copy: ValidateCopy, log_func: F) -> Result<String, String>
    where
        F: FnMut(&str),
    {
        let log_func = Arc::new(Mutex::new(log_func));

        if server.is_empty() {
            let msg = "exec runnable program failed, one of `host`、`port`、`username` and `password` server items is empty !";
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        if copy.is_empty() {
            let msg = "exec runnable program failed, one of `hash`、`file_dir` and `dest_dir` copy items is empty !";
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        let file_path = Path::new(&copy.file_path);

        // 判断文件是否存在
        if !file_path.exists() {
            let msg = format!("exec runnable program failed, file path `{}` is not exists !", copy.file_path);
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 判断是否是文件
        if !file_path.is_file() {
            let msg = format!("exec runnable program failed, file path `{}` is not a file !", copy.file_path);
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 获取文件名
        let file_name = file_path.file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
        if file_name.is_empty() {
            let msg = "exec runnable program failed, can not get the filename !";
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 连接服务器
        let session = SftpHandler::connect(&server, log_func.clone())?;
        let sftp = session.sftp().map_err(|err| {
            let msg = format!("exec runnable program error: {:#?}", err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        // 文件校验并上传
        let dest_file_path = Self::validate_copy_file(&session, &sftp, &file_name, &server.username, &copy, log_func.clone())?;

        // 断开连接
        SftpHandler::close_session(session)?;
        Ok(dest_file_path)
    }

    /// 比较文件是否一致, 不一致则拷贝文件
    fn validate_copy_file<F>(session: &Session, sftp: &Sftp, file_name: &str, username: &str, copy: &ValidateCopy, log_func: Arc<Mutex<F>>) -> Result<String, String>
    where
        F: FnMut(&str),
    {
        SftpHandler::log_info("compare program ...", log_func.clone());

        // 放到用户主目录下
        let home_dir = SftpHandler::get_user_home_dir(&session, username)?;
        SftpHandler::log_info("user home dir: {}", log_func.clone());
        let dest_dir = Path::new(&home_dir).join(&copy.dest_dir);
        SftpHandler::log_info(&format!("server dest file dir: {:#?}", dest_dir), log_func.clone());

        // 判断目录是否存在
        SftpHandler::check_dir(&sftp, &dest_dir.as_path().to_string_lossy().to_string(), log_func.clone())?;

        // 获取服务器文件的 hash 值
        let dest_file_path = dest_dir.join(&file_name).as_path().to_string_lossy().to_string();
        let is_hash_equal = match SftpHandler::get_file_hash(&sftp, &dest_file_path) {
            Ok(remote_file_hash) => {
                SftpHandler::log_info(&format!("server dest file hash: {}, file hash: {}", &remote_file_hash, &copy.hash), log_func.clone());

                // 比较 hash 是否一致
                if remote_file_hash.trim() == copy.hash.trim() {
                    true
                } else {
                    false
                }
            }
            Err(_) => {
                SftpHandler::log_error(&format!("can not get file `{}` hash !", &dest_file_path), log_func.clone());
                false
            }
        };

        // 如果程序存存在, 则判断是否已启动
        let mut pid = String::new();
        if sftp.stat(Path::new(&dest_file_path)).is_ok() {
            pid = Self::judge_program_running(&session, &file_name, log_func.clone())?;
        }

        SftpHandler::log_info(&format!("program pid: {}", pid), log_func.clone());
        // 上传文件
        if !is_hash_equal {
            SftpHandler::log_info(&format!("begin to upload file: {}", &file_name), log_func.clone());

            // 如果程序启动则结束进行
            if !pid.is_empty() {
                Self::kill_pid(&session, &pid)?;
            }

            SftpHandler::upload(&sftp, &copy.file_path, &dest_dir.as_path().to_string_lossy().to_string(), &file_name, log_func.clone())?;
            SftpHandler::log_info(&format!("upload file `{}` success", &file_name), log_func.clone());
        } else {
            SftpHandler::log_info("compare program no different !", log_func.clone());
        }

        SftpHandler::log_info("compare program success ...", log_func.clone());
        Ok(dest_file_path)
    }

    /// 运行程序, 如果程序已被杀死, 或者没有被杀死且 pid 为空, 则 启动程序
    pub fn exec_program<F, D>(sess: Option<Session>, server: &Server, dest_file_path: &str, secs: Option<u64>, func: F, log_func: D) -> Result<(), String>
    where
        F: FnMut(&str),
        D: FnMut(&str),
    {
        let log_func = Arc::new(Mutex::new(log_func));

        if server.is_empty() {
            let msg = "exec runnable program failed, one of `host`、`port`、`username` and `password` server items is empty !";
            info!("{}", msg);
            return Err(Error::convert_string(&msg));
        }

        // 连接服务器
        let session = if let Some(sess) = sess { sess } else { SftpHandler::connect(&server, log_func.clone())? };

        let sftp = session.sftp().map_err(|err| {
            let msg = format!("exec runnable program error: {:#?}", err);
            Error::convert_string(&msg);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        // 判断程序是否在运行
        let mut pid = String::new();
        let file_name = Path::new(&dest_file_path).file_name().unwrap_or(OsStr::new("")).to_string_lossy().to_string();
        if sftp.stat(Path::new(&dest_file_path)).is_ok() {
            pid = SftpRunnableHandler::judge_program_running(&session, &file_name, log_func.clone())?;
        }

        // 如果在运行，则直接结束
        if !pid.is_empty() {
            SftpRunnableHandler::kill_pid(&session, &pid)?;
        }

        let time = if let Some(secs) = secs { secs } else { 1 };

        info!("start program {} ...", dest_file_path);
        let func = Arc::new(Mutex::new(func));
        let mut channel = SftpHandler::create_channel(&session)?;

        // 通道一直会开着的, 因为要监听程序的输出, 当通道关闭后, 程序也结束
        channel.exec(dest_file_path).map_err(|err| {
            let msg = format!("start program `{}` error: {:#?}", dest_file_path, err);
            error!("{}", &msg);
            SftpHandler::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        let mut stdout = channel.stream(0); // 0表示标准输出
        let mut buffer = [0; 4096];
        loop {
            let bytes = match stdout.read(&mut buffer) {
                Ok(bytes) => Some(bytes),
                Err(_) => None,
            };

            if bytes.is_none() {
                break;
            }

            let bytes = bytes.unwrap();
            if bytes == 0 {
                break;
            }

            // 处理输出，可以根据需要自定义逻辑
            let output = String::from_utf8_lossy(&buffer[..bytes]);
            info!("{}", output);

            // 执行函数
            let mut exec_func = func.lock().unwrap();
            (*exec_func)(&output.to_string());
            thread::sleep(Duration::from_secs(time));
        }

        Ok(())
    }

    /// 判断程序是否已启动 `ps aux | grep xxx | grep -v grep`
    pub fn judge_program_running<F>(session: &Session, file_name: &str, log_func: Arc<Mutex<F>>) -> Result<String, String>
    where
        F: FnMut(&str),
    {
        SftpHandler::log_info("judge program running", log_func.clone());
        let mut channel = SftpHandler::create_channel(&session)?;

        let cmd = format!("ps aux | grep {} | grep -v grep", file_name);
        SftpHandler::log_info(&format!("judge program running command: {}", cmd), log_func.clone());
        channel.exec(&cmd).map_err(|err| {
            let msg = format!("grep process `{}` error: {:#?}", file_name, err);
            error!("{}", &msg);
            SftpHandler::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        let mut output = String::new();
        channel.read_to_string(&mut output).map_err(|err| {
            let msg = format!("read grep command error: {:#?}", err);
            error!("{}", &msg);
            SftpHandler::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        SftpHandler::log_info(&format!("judge program running output: {}", output), log_func.clone());
        let pid: Option<&str> = output.lines().filter(|line| line.contains(file_name) && !line.contains("grep")).next().and_then(|line| line.split_whitespace().nth(1));
        SftpHandler::close_channel(&mut channel)?;
        if let Some(pid) = pid {
            return Ok(pid.to_string());
        }

        return Ok(String::new());
    }

    /// 杀掉进程
    pub fn kill_pid(session: &Session, pid: &str) -> Result<(), String> {
        let mut channel = SftpHandler::create_channel(session)?;

        let cmd = format!("kill {}", pid);
        channel.exec(&cmd).map_err(|err| {
            let msg = format!("kill process pid `{}` error: {:#?}", pid, err);
            error!("{}", &msg);
            SftpHandler::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        SftpHandler::close_channel(&mut channel)?;
        Ok(())
    }
}
