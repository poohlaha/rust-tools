//! sftp

use crate::config::Server;
use crypto_hash::{hex_digest, Algorithm};
use handlers::error::Error;
use handlers::file::FileHandler;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info};
use ssh2::{Channel, FileStat, Session, Sftp};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct SftpHandler;

const DEFAULT_TIMEOUT: u64 = 10;

impl SftpHandler {
    /// 连接服务器
    pub fn connect<F>(server: &Server, log_func: Arc<Mutex<F>>) -> Result<Session, String>
    where
        F: FnMut(&str),
    {
        let address = format!("{}:{}", &server.host, server.port);
        let socket = SocketAddr::from_str(&address).map_err(|err| {
            let msg = format!("convert {} to socket address error: {:#?}", &address, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        let timeout = Self::get_time_out(server.timeout);
        Self::log_info(&format!("connect timeout: {:#?}", timeout), log_func.clone());

        Self::log_info("create tcp ..", log_func.clone());
        let tcp = TcpStream::connect_timeout(&socket, timeout).map_err(|err| {
            let msg = format!("connect to {} error: {:#?}", &address, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        Self::log_info("create session ..", log_func.clone());
        let mut session = Session::new().map_err(|err| {
            let msg = format!("get session error: {:#?}", err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        session.set_tcp_stream(tcp);

        Self::log_info("session handshake ..", log_func.clone());
        session.handshake().map_err(|err| {
            let msg = format!("connect to {} error: {:#?}", &address, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        Self::log_info("session auth ..", log_func.clone());
        session.userauth_password(&server.username, &server.password).map_err(|err| {
            let msg = format!("auth {} `user` and `password` error: {:#?}", &address, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        if !session.authenticated() {
            let msg = format!("authentication server: {} failed !", &address);
            error!("{}", &msg);
            return Err(Error::convert_string(&msg));
        }

        Self::log_info(&format!("connect {} success !", &address), log_func.clone());
        Ok(session)
    }

    /// 文件上传
    pub(crate) fn upload<F>(sftp: &Sftp, file_path: &str, dest_dir: &str, file_name: &str, log_func: Arc<Mutex<F>>) -> Result<(), String>
    where
        F: FnMut(&str),
    {
        if !Path::new(file_path).exists() {
            let msg = format!("upload dir failed, file path: {} not exists !", &file_path);
            error!("{}", &msg);
            return Err(Error::convert_string(&msg));
        }

        // 判断目录是否存在, 不存在则创建
        Self::check_dir(sftp, dest_dir, log_func.clone())?;
        let remote_file_path = Path::new(dest_dir).join(file_name);
        let remote_file_path_str = remote_file_path.as_path().to_string_lossy().to_string();

        // 判断文件是否存在, 存在则删除
        if sftp.stat(remote_file_path.as_path()).is_ok() {
            sftp.unlink(&remote_file_path).map_err(|err| {
                let msg = format!("delete file `{}` error: {:#?}", &remote_file_path_str, err);
                error!("{}", &msg);
                Error::convert_string(&msg)
            })?;
        }

        let mut remote_file = sftp.create(&remote_file_path).map_err(|err| {
            let msg = format!("upload file failed, create file `{}` error: {:#?}", &remote_file_path_str, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        Self::log_info(&format!("uploading file {} ...", file_path), log_func.clone());

        // progress bar
        let pb = ProgressBar::new_spinner();
        // pb.enable_steady_tick(Duration::from_millis(120));
        pb.set_style(ProgressStyle::with_template("{spinner:.blue} {msg}").unwrap().tick_strings(&["▹▹▹▹▹", "▸▹▹▹▹", "▹▸▹▹▹", "▹▹▸▹▹", "▹▹▹▸▹", "▹▹▹▹▸", "▪▪▪▪▪"]));
        pb.set_message(format!("Uploading {}...", file_path));

        let buffer = FileHandler::read_file_buffer(file_path)?;
        remote_file.write_all(&buffer).map_err(|err| {
            let msg = format!("upload file `{}` error: {:#?}", file_path, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        pb.finish_with_message(format!("Upload File {} Success !", file_path));

        // upload success
        Self::log_info(&format!("upload file `{}` success, file path: {}", file_name, &remote_file_path_str), log_func.clone());

        // 设置文件权限
        Self::log_info(&format!("begin to set file `{}` permission ...", file_path), log_func.clone());
        sftp.setstat(
            &remote_file_path,
            FileStat {
                size: None,
                uid: None,
                gid: None,
                perm: Some(0o777),
                atime: None,
                mtime: None,
            },
        )
        .map_err(|err| {
            let msg = format!("set file permission `{}` error: {:#?}", &remote_file_path_str, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        Self::log_info(&format!("set file `{}` permission success !", file_name), log_func.clone());
        Ok(())
    }

    /// 判断目录是否存在, 不存在则创建
    pub(crate) fn check_dir<F>(sftp: &Sftp, file_path: &str, log_func: Arc<Mutex<F>>) -> Result<(), String>
    where
        F: FnMut(&str),
    {
        let path = Path::new(file_path);

        // 目录存在
        if sftp.stat(&path).is_ok() {
            return Ok(());
        }

        // 不存在则创建
        Self::log_info(&format!("remote file dir: `{}` is not exists, it will be created!", file_path), log_func.clone());
        sftp.mkdir(&path, 0o777).map_err(|err| {
            let msg = format!("mkdir file path `{}` error: {:#?}", file_path, err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        Ok(())
    }

    /// 获取超时时间
    fn get_time_out(timeout: Option<u64>) -> Duration {
        if let Some(timeout) = timeout {
            if timeout > 0 {
                return Duration::from_secs(timeout);
            }
        }

        return Duration::from_secs(DEFAULT_TIMEOUT);
    }

    /// 获取运程文件 hash 值
    pub(crate) fn get_file_hash(sftp: &Sftp, file_path: &str) -> Result<String, String> {
        // 文件不存在
        if !sftp.stat(Path::new(file_path)).is_ok() {
            info!("remote file path: `{}` is not exists!", file_path);
            return Ok(String::new());
        }

        let mut file = sftp.open(Path::new(file_path)).map_err(|err| {
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

    /// 获取用户主目录
    pub(crate) fn get_user_home_dir(session: &Session, username: &str) -> Result<String, String> {
        let mut channel = Self::create_channel(&session)?;

        channel.exec(&format!("getent passwd {}", username)).map_err(|err| {
            let msg = format!("get user `{}` home dir error: {:#?}", username, err);
            error!("{}", &msg);
            Self::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        let mut output = String::new();
        channel.read_to_string(&mut output).map_err(|err| {
            let msg = format!("read grep command error: {:#?}", err);
            error!("{}", &msg);
            Self::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        Self::close_channel(&mut channel)?;

        let fields: Vec<&str> = output.trim().split(':').collect();
        if fields.len() >= 6 {
            return Ok(fields[5].to_string());
        }

        return Err(Error::convert_string(&format!("get user `{}` home dir failed !", username)));
    }

    /// 创建 channel
    pub fn create_channel(session: &Session) -> Result<Channel, String> {
        let channel = session.channel_session().map_err(|err| {
            let msg = format!("compare program failed, create `channel session` error: {:#?}", err);
            Error::convert_string(&msg);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        Ok(channel)
    }

    pub fn close_channel_in_err(channel: &mut Channel) {
        match Self::close_channel(channel) {
            Ok(_) => {}
            Err(err) => {
                error!("{}", err)
            }
        };
    }

    /// 获取 channel 输出信息
    pub fn get_channel_output(channel: &mut Channel) -> Result<(String, String), String> {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        channel.read_to_end(&mut stdout).map_err(|err| {
            let msg = format!("read grep command error: {:#?}", err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        channel.stderr().read_to_end(&mut stderr).map_err(|err| {
            let msg = format!("read grep command error: {:#?}", err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        return Ok((String::from_utf8_lossy(&stdout).to_string(), String::from_utf8_lossy(&stderr).to_string()));
    }

    /// 关闭 channel
    pub fn close_channel(channel: &mut Channel) -> Result<(), String> {
        info!("close channel ...");
        channel.send_eof().map_err(|err| {
            let msg = format!("close channel error: {:#?}", err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        channel.wait_eof().map_err(|err| {
            let msg = format!("close channel error: {:#?}", err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        channel.close().map_err(|err| {
            let msg = format!("close channel error: {:#?}", err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        info!("close channel success !");
        Ok(())
    }

    /// 关闭 session
    pub fn close_session(session: Session) -> Result<(), String> {
        session.disconnect(None, "Bye bye !", None).map_err(|err| {
            let msg = format!("close session error: {:#?}", err);
            error!("{}", &msg);
            Error::convert_string(&msg)
        })?;

        Ok(())
    }

    /// 记录日志
    pub fn log_info<F>(msg: &str, log_func: Arc<Mutex<F>>)
    where
        F: FnMut(&str),
    {
        info!("{}", msg);
        let mut log_func = log_func.lock().unwrap();
        (*log_func)(msg);
    }

    /// 记录日志
    pub fn log_error<F>(msg: &str, log_func: Arc<Mutex<F>>)
    where
        F: FnMut(&str),
    {
        error!("{}", msg);
        let mut log_func = log_func.lock().unwrap();
        (*log_func)(msg);
    }
}
