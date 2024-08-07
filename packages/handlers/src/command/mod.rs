//! 通过 `Command::new` 命令运行

use log::info;
use std::io::{BufRead, BufReader};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex};
use std::{io, thread};

pub struct CommandHandler;

impl CommandHandler {
    /// 执行命令, 获取输出结果
    pub fn exec_command_result(command: &str) -> String {
        let output = Self::get_exec_command_output(command);
        if let Some(output) = output {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).to_string();
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                info!("exec command error:\n");
                for line in stderr.lines() {
                    info!("{}", line);
                }
            }
        }

        return String::new();
    }

    /// 执行命令
    pub fn exec_command(command: &str) -> (bool, Vec<String>) {
        let output = Self::get_exec_command_output(command);
        let mut flag = false;
        let mut lines: Vec<String> = Vec::new();
        if let Some(output) = output {
            if output.status.success() {
                let output_str = String::from_utf8_lossy(&output.stdout).to_string();
                for line in output_str.lines() {
                    info!("{}", line);
                    lines.push(line.to_string());
                }
                flag = true;
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                info!("exec command error:\n");
                for line in stderr.lines() {
                    info!("{}", line);
                    lines.push(line.to_string());
                }
                flag = false;
            }
        }

        return (flag, lines);
    }

    /// 运行命令, 输出实时日志
    pub fn get_command_lines(command: &str) -> Vec<String> {
        if command.is_empty() {
            info!("command is empty !");
            return Vec::new();
        }

        let _command = command.replace("\n", " && ");
        let child: io::Result<Child>;
        // windows 通过 cmd /C 执行多条命令: cd c:\\usr\\local\\nginx\\sbin/ && nginx
        #[cfg(target_os = "windows")]
        {
            info!("exec command:\n {}", _command);
            child = Command::new("cmd").args(&["/C", &_command]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
        }

        // linux|macos 通过 shell -c 执行多条命令: cd /usr/local/nginx/sbin/\n./nginx
        #[cfg(target_os = "macos")]
        {
            info!("exec command:\n {}", command);
            child = Command::new("sh").arg("-c").arg(command).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
        }

        #[cfg(target_os = "linux")]
        {
            info!("exec command:\n {}", command);
            child = Command::new("sh").arg("-c").arg(command).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
        }

        return Self::get_exec_command_real_time_output_by_spawn(child);
    }

    /**
    &&: 用于在前一条命令成功执行后才执行下一条命令。如果前一条命令返回零（表示成功），则才会执行后续的命令。如果前一条命令返回非零（表示失败），则后续的命令将被跳过
    \n: \n 或分号（;）用于按顺序执行多个命令，无论前一条命令是否成功。每个命令都会按顺序执行，不考虑前一条命令的执行状态。
     */
    fn get_exec_command_output(command: &str) -> Option<Output> {
        if command.is_empty() {
            info!("command is empty !");
            return None;
        }

        let output: Result<Output, io::Error>;
        let _command = command.replace("\n", " && ");

        // windows 通过 cmd /C 执行多条命令: cd c:\\usr\\local\\nginx\\sbin/ && nginx
        #[cfg(target_os = "windows")]
        {
            info!("exec command:\n {}", _command);
            output = Command::new("cmd").args(&["/C", &_command]).output();
        }

        // linux|macos 通过 shell -c 执行多条命令: cd /usr/local/nginx/sbin/\n./nginx
        #[cfg(target_os = "macos")]
        {
            info!("exec command:\n {}", _command);
            output = Command::new("sh").arg("-c").arg(command).output()
        }

        #[cfg(target_os = "linux")]
        {
            info!("exec command:\n {}", _command);
            output = Command::new("sh").arg("-c").arg(command).output()
        }

        return match output {
            Ok(output) => Some(output),
            Err(err) => {
                info!("exec command error: {:?}", err);
                None
            }
        };
    }

    /// 通过 output 实时输出日志
    pub fn get_exec_command_real_time_output_by_spawn(mut spawn: io::Result<Child>) -> Vec<String> {
        let child = match spawn.as_mut() {
            Ok(child) => Some(child),
            Err(err) => {
                info!("filed to get spawn, error: {:#?}", err);
                None
            }
        };

        if child.is_none() {
            return Vec::new();
        }

        let mut child = spawn.unwrap();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        if stdout.is_none() {
            info!("failed to open stdout !");
            return Vec::new();
        }

        if stderr.is_none() {
            info!("failed to open stderr !");
            return Vec::new();
        }

        let stdout = stdout.unwrap();
        let stderr = stderr.unwrap();
        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);
        let last_line = Arc::new(Mutex::new(String::new()));
        let last_line_clone = Arc::clone(&last_line); // 克隆 Arc<Mutex<>>，以便在线程内部使用

        // let lines: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new())); // 使用读写锁, RwLock 为 `多读取少写`, 允许多个线程同时访问, 因此用 Mutex
        let lines: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        // Arc（共享指针）确保多个引用可以同时访问相同的数据，但它们仍然引用相同的数据
        let lines_clone = lines.clone();

        // 启动两个线程来实时输出 stdout 和 stderr
        let stdout_thread = thread::spawn(move || {
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    info!("{}", line);

                    /*
                    let mut lines_guard = lines_clone.write().unwrap();
                    lines_guard.push(line.clone());
                     */

                    let mut line_guard = lines_clone.lock().unwrap();
                    line_guard.push(line.clone());
                    // drop(line_guard); // 释放 Mutex 共享资源

                    // 将最后一行保存为变量
                    if !line.trim().is_empty() {
                        let mut last_line = last_line_clone.lock().unwrap();
                        *last_line = line.clone();
                    }
                }
            }
        });

        let stderr_thread = thread::spawn(move || {
            for line in stderr_reader.lines() {
                if let Ok(line) = line {
                    info!("{}", line);
                }
            }
        });

        // 等待子进程完成
        let status = match child.wait() {
            Ok(status) => Some(status),
            Err(err) => {
                info!("filed to wait spawn finished, error: {:#?}", err);
                None
            }
        };

        if status.is_none() {
            return Vec::new();
        }

        let status = status.unwrap();
        match stdout_thread.join() {
            Ok(_) => {}
            Err(err) => {
                info!("filed to wait stdout thread finished, error: {:#?}", err);
            }
        }

        match stderr_thread.join() {
            Ok(_) => {}
            Err(err) => {
                info!("filed to wait stderr thread finished, error: {:#?}", err);
            }
        }

        // 判断是否有 SUCCESSFUL 字段
        let success = status.success();
        info!("success: {}", success);

        // let lines_read = lines.read().unwrap(); // Rwlock
        let lines_read = lines.lock().unwrap(); // lines_clone 是一个普通的 Arc 智能指针的克隆
        let lines = lines_read.clone();
        if !success {
            return Vec::new();
        }

        return lines.clone();
    }
}
