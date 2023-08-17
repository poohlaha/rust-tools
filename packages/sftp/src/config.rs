//! 配置

/// 服务器配置
#[derive(Debug, Default)]
pub struct Server {
    pub host: String,
    pub port: u32,
    pub username: String,
    pub password: String
}

impl Server {
    pub fn is_empty(&self) -> bool {
        return self.host.is_empty() && self.port == 0 && self.username.is_empty() && self.password.is_empty();
    }
}

/// 文件上传配置
#[derive(Debug, Default)]
pub struct Upload {
    pub cmds: Vec<String>, // 服务端命令
    pub dir: String, // 目录 或 文件 名称
    pub server_dir: String, // 上传服务器目录
    pub server_file_name: Option<String> // 服务端文件名称, 如果是文件默认同文件名, 如果是目录，默认同目录名
}

impl Upload {
    pub fn is_empty(&self) -> bool {
        return self.cmds.is_empty() && self.dir.is_empty() && self.server_dir.is_empty();
    }
}