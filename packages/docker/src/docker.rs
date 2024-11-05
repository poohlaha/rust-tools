//! Docker, 可以使用第三方库 `bollard`

use crate::error::Error;
use crate::DockerConfig;
use handlers::command::func::CommandFuncHandler;
use handlers::command::CommandHandler;
use handlers::file::FileHandler;
use handlers::utils::Utils;
use log::{error, info};
use sftp::sftp::SftpHandler;
use ssh2::Session;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};

pub struct DockerHandler;

impl DockerHandler {
    pub async fn exec<F>(docker_config: &DockerConfig, server: &sftp::config::Server, func: F) -> Result<bool, String>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let msg = format!("docker config: {:#?}", docker_config);
        func(&msg);

        if DockerConfig::is_empty(&docker_config) {
            return Err(Error::convert_string("run pipeline failed, `docker config some field` is empty!"));
        }

        // 判断本机有没有安装docker
        let success = CommandHandler::check_installed_command("docker");
        if !success {
            return Err(Error::convert_string("no `docker` installed in os !"));
        }

        // 判断 docker 是否已启动
        let str = CommandHandler::exec_command_result("docker info");
        if str.is_empty() {
            return Err(Error::convert_string("`docker` is not running !"));
        }

        // 获取 docker pull | docker push 命令
        let mut commands: Vec<String> = Vec::new();
        commands.push(format!("cd {}", docker_config.dir));

        let time = Utils::get_date(Some("%Y%m%d%H%M%S".to_string()));

        // 创建 nginx.conf 文件
        let nginx_file_name = format!("nginx_{}.conf", time); // nginx 文件名
        let nginx_file_path = Path::new(&docker_config.dir).join(&nginx_file_name);
        let nginx_file_path_str = nginx_file_path.to_string_lossy().to_string();
        FileHandler::write_to_file_when_clear(&nginx_file_path_str, &docker_config.nginx_content)?;

        let mut dockerfile_content = docker_config.dockerfile.clone();

        // 添加 nginx
        if !docker_config.nginx_path.is_empty() && !docker_config.nginx_content.is_empty() {
            let mut content: Vec<String> = dockerfile_content.lines().map(String::from).collect();
            content.push(format!("ADD {} {}", nginx_file_name, docker_config.nginx_path));

            if !docker_config.deploy_dir.is_empty() {
                content.push(format!("COPY {} ./", docker_config.deploy_dir));
            }

            dockerfile_content = content.join("\n");
        }

        dockerfile_content.push_str("\n CMD [\"nginx\", \"-g\", \"daemon off;\"]");

        // 创建 Dockerfile 文件
        let dockerfile_file_name = format!("Dockerfile_{}", time); // dockerfile 文件名
        let dockerfile_file_path = Path::new(&docker_config.dir).join(&dockerfile_file_name);
        let dockerfile_file_path_str = dockerfile_file_path.to_string_lossy().to_string();
        FileHandler::write_to_file_when_clear(&dockerfile_file_path_str, &dockerfile_content)?;
        let msg = format!("Dockerfile content: \n{:#?}", dockerfile_content);
        func(&msg);

        let image = format!("{}/{}/{}:{}", docker_config.address, docker_config.namespace, docker_config.image, docker_config.version);

        // 判断是否有 buildx 命令, 如果没有直接用 build 就行
        let str = CommandHandler::exec_command_result("docker buildx version");
        let mut docker_buildx = "buildx";
        if str.is_empty() {
            docker_buildx = "";
            info!("docker `buildx` not found, use docker `build` !")
        }

        if docker_config.need_push == "Yes" {
            let pull_nginx_command_list = Self::exec_docker_pull_nginx(&docker_config);
            if pull_nginx_command_list.is_empty() {
                FileHandler::delete_file(&dockerfile_file_path_str)?; // 删除 Dockerfile 文件
                FileHandler::delete_file(&nginx_file_path_str)?; // 删除 nginx.conf 文件
                return Err(Error::convert_string("can not get pull nginx command !"));
            }

            commands.push(format!("docker login {} --username {} --password {}", docker_config.address, docker_config.user, docker_config.password));
            // pull command list
            for pull_nginx_command in pull_nginx_command_list.iter() {
                commands.push(pull_nginx_command.to_string());
            }
            commands.push(format!("docker {} build --file ./{} -t {} --platform {} -o type=docker .", docker_buildx, dockerfile_file_name, image, docker_config.platform));
            commands.push(format!("docker push {}", image));
        } else {
            // 不需要推送，直接打本地包
            commands.push(format!(
                "docker {} build --file ./{} -t {}:{} --platform {} -o type=docker .",
                docker_buildx, dockerfile_file_name, docker_config.image, docker_config.version, docker_config.platform
            ));
        }

        let msg = format!("docker commands:\n{:#?}", commands);
        func(&msg);

        let func_cloned = Arc::new(RwLock::new(func));
        for command in commands.iter() {
            let func_clone = func_cloned.clone();
            let success = CommandFuncHandler::exec_command(&command, &docker_config.dir, move |msg| {
                let func = func_clone.read().unwrap();
                (*func)(&msg);
            });

            if !success {
                FileHandler::delete_file(&dockerfile_file_path_str)?; // 删除 Dockerfile 文件
                FileHandler::delete_file(&nginx_file_path_str)?; // 删除 nginx.conf 文件
                return Err(Error::convert_string(&format!("run docker command failed: {}", command)));
            }
        }

        FileHandler::delete_file(&dockerfile_file_path_str)?; // 删除 Dockerfile 文件
        FileHandler::delete_file(&nginx_file_path_str)?; // 删除 nginx.conf 文件

        info!("run docker commands success !");
        if docker_config.need_push == "Yes" {
            let func_cloned = func_cloned.clone();
            return Self::update_image(
                &docker_config,
                &image,
                move |msg| {
                    let func = func_cloned.read().unwrap();
                    (*func)(msg);
                },
                server,
            )
            .await;
        }

        return Ok(true);
    }

    //  拉取 nginx 镜像 docker pull xxx
    fn exec_docker_pull_nginx(docker_config: &DockerConfig) -> Vec<String> {
        let mut file_lines: Vec<String> = Vec::new();
        let lines = docker_config.dockerfile.lines();
        for line in lines.into_iter() {
            if line.is_empty() {
                continue;
            }

            if line.starts_with("FROM ") {
                file_lines.push(line.to_string());
            }
        }

        info!("docker from list: {:#?}", file_lines);

        if file_lines.is_empty() {
            return Vec::new();
        }

        let mut commands: Vec<String> = Vec::new();
        for line in file_lines.iter() {
            if !line.starts_with("FROM ") {
                continue;
            }

            let command = line.split_whitespace().nth(1).map(String::from).unwrap_or(String::new());
            info!("docker pull command: {}", command);
            if !command.is_empty() {
                commands.push("docker pull".to_string() + &command);
            }
        }

        return commands;
    }

    /// 连接服务器, 修改 image 地址
    async fn update_image<F>(docker_config: &DockerConfig, image: &str, func: F, server: &sftp::config::Server) -> Result<bool, String>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let func_cloned = Arc::new(Mutex::new(func));

        {
            let msg = "update `image` in `kubectl` ...";
            let func_clone = func_cloned.clone();
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        let log_func = |_: &str| {};
        let log_func = Arc::new(Mutex::new(log_func));
        let session = SftpHandler::connect(&server, log_func.clone())?;

        // 登录到 root
        let login_cmd = format!("echo {} | sudo -S -i", server.password);

        // 获取当前 YAML 配置
        let yaml_cmd = format!("kubectl get deploy {} -n {} -o yaml", docker_config.image, docker_config.kubernetes_namespace);
        let cmd = format!("{} bash -c '{}'", login_cmd, yaml_cmd);

        {
            let msg = format!("get yaml config command: {}", cmd);
            let func_clone = func_cloned.clone();
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        let func_clone = func_cloned.clone();
        let yaml_content = Self::exec_remote_command(&session, &cmd, "get kubectl yaml config error", move |msg| {
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        })?;

        /*
        let regex = Regex::new(r#"image:\s*([^\s]+)"#).unwrap();
        let modified_yaml = regex.replace_all(&yaml_content, format!("image: {}", image).as_str());
        */

        {
            let msg = format!("kubectl yaml content:\n{}", yaml_content);
            let func_clone = func_cloned.clone();
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        if yaml_content.is_empty() {
            return Err(Error::convert_string("can not get kubectl yaml config !"));
        }

        let image_cmd = format!(
            r#"{{"spec": {{
                "template": {{
                    "spec": {{
                        "containers": [
                            {{
                                "name": "{}",
                                "image": "{}",
                                 "imagePullPolicy": "Always"
                            }}
                        ]
                    }}
                }}
            }}
        }}"#,
            docker_config.image, image
        )
        .replace("\n", "") // 去除所有换行符
        .replace("  ", ""); // 去除多余空格

        let cmd = format!("{} kubectl patch deployment {} -n {} --type=merge --patch '{}'", login_cmd, docker_config.image, docker_config.kubernetes_namespace, image_cmd);

        {
            let msg = format!("kubectl update image command:\n{}", cmd);
            let func_clone = func_cloned.clone();
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        let func_clone = func_cloned.clone();
        let output = Self::exec_remote_command(&session, &cmd, "exec command `kubectl patch` error", move |msg| {
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        })?;

        {
            let msg = format!("kubectl patch output info: {}", output);
            let func_clone = func_cloned.clone();
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        if output.is_empty() {
            return Err(Error::convert_string("update `image` in `kubectl` failed ..."));
        }

        {
            let msg = "update `image` in `kubectl` success ...";
            let func_clone = func_cloned.clone();
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        // 如果没有 change, 需要删除原来的 pod
        if output.contains("no change") {
            {
                let msg = "no change will delete pod ...";
                let func_clone = func_cloned.clone();
                let func = func_clone.lock().unwrap();
                (*func)(&msg);
            }

            let func_clone = func_cloned.clone();
            let success = Self::delete_pod_name(&session, docker_config, &login_cmd, move |msg| {
                let func = func_clone.lock().unwrap();
                (*func)(&msg);
            })?;

            if !success {
                return Err(Error::convert_string("delete pod error!"));
            }

            {
                let msg = "delete pod success ...";
                let func_clone = func_cloned.clone();
                let func = func_clone.lock().unwrap();
                (*func)(&msg);
            }
        }

        // 重启 pod: kubectl rollout restart deployment/xxx(image) -n xxx
        let cmd = format!("{} kubectl rollout restart deployment/{} -n {}", login_cmd, docker_config.image, docker_config.kubernetes_namespace);

        {
            let msg = format!("pod restart command: {}", cmd);
            let func_clone = func_cloned.clone();
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        let func_clone = func_cloned.clone();
        let output = Self::exec_remote_command(&session, &cmd, "exec command `kubectl patch` error", move |msg| {
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        })?;

        {
            let msg = format!("pod restart command output info: {}", output);
            let func_clone = func_cloned.clone();
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        if output.is_empty() {
            return Ok(false);
        }

        return Ok(true);
    }

    fn delete_pod_name<F>(session: &Session, docker_config: &DockerConfig, login_cmd: &str, func: F) -> Result<bool, String>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let func_cloned = Arc::new(Mutex::new(func));

        // 1. 查找 pod 名字 kubectl get pod -n xxx | grep xxx
        let cmd = format!("{} kubectl get pod -n {} | grep {}", login_cmd, docker_config.kubernetes_namespace, docker_config.image);
        let func_clone = func_cloned.clone();
        let output = Self::exec_remote_command(session, &cmd, "kubectl get pod name error", move |msg| {
            let func = func_clone.lock().unwrap();
            (*func)(msg);
        })?;

        if output.is_empty() {
            {
                let msg = "no pod name output";
                let func_clone = func_cloned.clone();
                let func = func_clone.lock().unwrap();
                (*func)(msg);
            }
            return Ok(false);
        }

        let mut pod_name = String::new();
        if let Some(line) = output.lines().find(|line| line.starts_with(&docker_config.image)) {
            let name = line.split_whitespace().next().unwrap_or("");
            {
                let msg = format!("pod name: {}", name);
                let func_clone = func_cloned.clone();
                let func = func_clone.lock().unwrap();
                (*func)(&msg);
            }
            pod_name = name.to_string()
        } else {
            {
                let msg = "no pad name get !";
                let func_clone = func_cloned.clone();
                let func = func_clone.lock().unwrap();
                (*func)(&msg);
            }
        }

        if pod_name.is_empty() {
            return Ok(false);
        }

        // 2. delete pod: kubectl delete pod -n xxx ${podname}
        let cmd = format!("{} kubectl delete pod -n {} {}", login_cmd, docker_config.kubernetes_namespace, pod_name);
        let func_clone = func_cloned.clone();
        let mut output = Self::exec_remote_command(session, &cmd, "kubectl delete pod error", move |msg| {
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        })?;

        {
            let msg = format!("kubectl delete pod output: {}", output);
            let func_clone = func_cloned.clone();
            let func = func_clone.lock().unwrap();
            (*func)(&msg);
        }

        output = output.trim().to_string();

        if output.ends_with("deleted") {
            return Ok(true);
        }

        return Ok(false);
    }

    /// 执行远程命令
    fn exec_remote_command<F>(session: &Session, cmd: &str, error_msg: &str, func: F) -> Result<String, String>
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        let msg = format!("exec remote command: {}", cmd);
        func(&msg);

        let mut channel = SftpHandler::create_channel(&session)?;
        channel.exec(&cmd).map_err(|err| {
            let msg = format!("{}: {:#?}", error_msg, err);
            error!("{}", &msg);
            SftpHandler::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        let mut output = String::new();
        channel.read_to_string(&mut output).map_err(|err| {
            let msg = format!("{}: {:#?}", error_msg, err);
            error!("{}", &msg);
            SftpHandler::close_channel_in_err(&mut channel);
            Error::convert_string(&msg)
        })?;

        // PipelineRunnable::save_log(app, &format!("output info: {}", output), &pipeline.server_id, &pipeline.id, order);
        SftpHandler::close_channel_in_err(&mut channel);

        return Ok(output);
    }
}
