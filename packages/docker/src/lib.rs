pub mod docker;
pub mod error;

#[derive(Default, Debug, Clone)]
pub struct DockerConfig {
    pub dir: String,                  // 执行命令的目录
    pub dockerfile: String,           // Dockerfile 文件
    pub address: String,              // 远程仓库 address
    pub image: String,                // docker image
    pub version: String,              // docker image version
    pub namespace: String,            // 远程仓库 namespace
    pub user: String,                 // 远程仓库 用户名
    pub password: String,             // 远程仓库 密码
    pub need_push: String,            // 是否推送到远程仓库
    pub nginx_path: String,           // docker 中 nginx 地址
    pub nginx_content: String,        // docker 中 nginx 内容(如果需要拷贝 nginx)
    pub platform: String,             // docker 打包平台
    pub deploy_dir: String,           // 发布目录
    pub kubernetes_namespace: String, // Kubernetes 命名空间名称
}

impl DockerConfig {
    pub fn is_empty(config: &DockerConfig) -> bool {
        if config.need_push == "Yes" {
            return config.dir.is_empty()
                || config.dockerfile.is_empty()
                || config.image.is_empty()
                || config.address.is_empty()
                || config.namespace.is_empty()
                || config.user.is_empty()
                || config.password.is_empty()
                || config.platform.is_empty()
                || config.kubernetes_namespace.is_empty();
        }

        return config.dir.is_empty() || config.dockerfile.is_empty() || config.image.is_empty() || config.platform.is_empty();
    }
}
