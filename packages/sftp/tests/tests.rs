//! 测试 sftp

use sftp::config::{Server, Upload};
use sftp::runnable::SftpRunnableHandler;
use sftp::sftp::SftpHandler;
use sftp::upload::SftpUpload;

#[test]
fn test_sftp_upload() {
    let result = SftpUpload::exec(
        Server {
            host: "192.1.1.1".to_string(),
            port: 22,
            username: "test".to_string(),
            password: "test".to_string(),
            timeout: None,
        },
        Upload {
            cmds: vec![],
            dir: "/usr/local/test".to_string(),
            server_dir: "/usr/local/nginx/www/".to_string(),
            server_file_name: Some("test".to_string()),
            need_increment: false,
        },
        |str| {
            println!("{}", str);
        },
    )
    .unwrap();
    println!("result: \n {:#?}", result);
}

#[test]
fn test_run_program() {
    let server = Server {
        host: "192.1.1.1".to_string(),
        port: 22,
        username: "test".to_string(),
        password: "test".to_string(),
        timeout: None,
    };

    let session = SftpHandler::connect(&server).unwrap();
    SftpRunnableHandler::exec_program(Some(session), &server, "/usr/local/test/__MONITOR__/n-nacos-tools", None, |str| println!("{}", str)).unwrap();
}
