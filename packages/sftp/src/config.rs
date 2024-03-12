//! 配置

/// 服务器配置
#[derive(Debug, Default, Clone)]
pub struct Server {
    pub host: String,
    pub port: u32,
    pub username: String,
    pub password: String,
    pub timeout: Option<u64>,
}

impl Server {
    pub fn is_empty(&self) -> bool {
        return self.host.is_empty() || self.port == 0 || self.username.is_empty() || self.password.is_empty();
    }
}

/// 文件上传配置
#[derive(Debug, Default, Clone)]
pub struct Upload {
    pub cmds: Vec<String>,                // 服务端命令
    pub dir: String,                      // 目录 或 文件 名称
    pub server_dir: String,               // 上传服务器目录
    pub server_file_name: Option<String>, // 服务端文件名称, 如果是文件默认同文件名, 如果是目录，默认同目录名
    pub need_increment: bool,             // 是否增量发布, 如果是增量则需要比较文件是否一致
}

impl Upload {
    pub fn is_empty(&self) -> bool {
        return self.dir.is_empty() || self.server_dir.is_empty();
    }
}

// 校验拷贝文件
#[derive(Debug, Default, Clone)]
pub struct ValidateCopy {
    pub hash: String, // hash 值
    pub file_path: String, // 文件名全路径
    pub dest_dir: String, // 服务器文件目录
}

impl ValidateCopy {
    pub fn is_empty(&self) -> bool {
        return self.hash.is_empty() || self.file_path.is_empty() || self.dest_dir.is_empty();
    }
}

/// 文件上传返回结果
#[derive(Debug, Default, Clone)]
pub struct SftpUploadResult {
    pub file_count: u64, // 发布的文件个数
    pub exec_commands: Vec<String>, // 执行的命令集
    pub delete_file_count: u64, // 删除的文件个数
    pub need_increment: bool, // 是否增量发布
}