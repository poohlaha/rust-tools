//! 导出库

pub mod config;
pub mod sftp;
pub mod utils;

const LOGGER_PREFIX: &str = "[Rust Tools Sftp]: ";

#[cfg(test)]
mod tests {
    use crate::config::{Server, Upload};
    use super::*;

    #[test]
    fn test_upload() {
        let mut server_sftp = sftp::Sftp::new(
            Server {
                host: String::from("10.100.34.22"),
                port: 22,
                username: String::from("test"),
                password: String::from("test"),
                timeout: 0,
            },
            Upload {
                cmds: Vec::new(),
                dir: String::from("/usr/local/test/build"),
                server_dir: String::from("/usr/local/nginx/html/"),
                server_file_name: None,
                need_increment: false,
            }
        );

        server_sftp.send();
    }
}
