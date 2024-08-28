pub mod docker;
pub mod error;

#[derive(Default, Debug, Clone)]
pub struct DockerConfig {
    dir: String,                  // 执行命令的目录
    dockerfile: String,           // Dockerfile 文件
    address: String,              // 远程仓库 address
    image: String,                // docker image
    version: String,              // docker image version
    namespace: String,            // 远程仓库 namespace
    user: String,                 // 远程仓库 用户名
    password: String,             // 远程仓库 密码
    need_push: String,            // 是否推送到远程仓库
    nginx_path: String,           // docker 中 nginx 地址
    nginx_content: String,        // docker 中 nginx 内容(如果需要拷贝 nginx)
    platform: String,             // docker 打包平台
    deploy_dir: String,           // 发布目录
    kubernetes_namespace: String, // Kubernetes 命名空间名称
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
