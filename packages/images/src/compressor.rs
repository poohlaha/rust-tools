//! 图片压缩

use crate::factor::Factor;
use crate::img::Img;
use crossbeam_queue::SegQueue;
use fs_extra::dir;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::{fs, thread};
use colored::Colorize;

pub struct Compressor {
    pub factor: Factor,
    pub original_path: PathBuf,
    pub destination_path: PathBuf,
    pub thread_count: u32,
    pub image_size: u64,
}

#[derive(Debug)]
pub struct CompressorArgs {
    pub factor: Option<Factor>,
    pub origin: String,
    pub dest: String,
    pub thread_count: Option<u32>,
    pub image_size: u64, // 要压缩的图片最小值, 默认为 kb
}

pub struct CompressorFile {
    pub file_name: String,     // 文件名
    pub extension: String,     // 后缀
    pub file_stem: String,     // 后缀
    pub file_size: u64,        // 文件大小
    pub path: String,          // 全路径
    pub relative_path: String, // 相对路径
}

const FILE_LIST: [&str; 4] = ["jpg", "jpeg", "png", "gif"];
impl Compressor {
    pub fn new(args: CompressorArgs) -> Compressor {
        let factor = args.factor;
        let thread_count = args.thread_count;

        Compressor {
            factor: if factor.is_none() { Factor::default() } else { factor.clone().unwrap() },
            original_path: PathBuf::from(args.origin),
            destination_path: PathBuf::from(args.dest),
            thread_count: if factor.is_none() { 1 } else { thread_count.unwrap() },
            image_size: args.image_size,
        }
    }

    /// get compress dir file list
    fn get_origin_file_list(&self, file_path: &PathBuf, files: &mut Vec<CompressorFile>) {
        for entry in fs::read_dir(file_path).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                self.get_origin_file_list(&path, files)
            } else {
                let relative_path = path.strip_prefix(&self.original_path).unwrap().to_str().unwrap();
                let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                // let file_stem = PathBuf::from(file_name.clone()).file_stem().unwrap().to_str().unwrap_or(""); // 文件前缀
                let extension = path.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
                let file_stem = file_name.clone().replace(&format!(".{}", extension), "");
                let size = fs::metadata(&path).unwrap().len();
                if self.image_size == 0 {
                    if FILE_LIST.contains(&extension) {
                        files.push(CompressorFile {
                            extension: extension.to_string(),
                            path: path.as_path().to_string_lossy().to_string(),
                            file_name: file_name.clone(),
                            file_stem: file_stem.to_string(),
                            file_size: size,
                            relative_path: relative_path.to_string(),
                        })
                    }

                    continue;
                }

                if size > self.image_size * 1024 {
                    if FILE_LIST.contains(&extension) {
                        files.push(CompressorFile {
                            extension: extension.to_string(),
                            path: path.as_path().to_string_lossy().to_string(),
                            file_name,
                            file_stem: file_stem.to_string(),
                            file_size: size,
                            relative_path: relative_path.to_string(),
                        })
                    }
                }
            }
        }
    }

    /// compress
    pub fn compress<F>(self, log_func: F) -> Result<bool, String>
    where
        F: FnMut(&str) + Send + 'static
    {
        let log_func = Arc::new(Mutex::new(log_func));

        if !self.original_path.exists() {
            let msg = format!("original path: {} is not exists", self.original_path.as_path().to_string_lossy().to_string().magenta().bold());
            log(&msg, log_func.clone());
            return Err(msg.to_string());
        }

        log(&format!("Starting compress {} ...", "images".cyan().bold()), log_func.clone());
        let start_time = Instant::now();

        let mut files: Vec<CompressorFile> = Vec::new();
        self.get_origin_file_list(&self.original_path, &mut files);
        log(&format!("total file count: {}", files.len().to_string().cyan().bold()), log_func.clone());

        if files.len() == 0 {
            let elapsed_time = format!("{:.2?}", start_time.elapsed()).magenta().bold();
            log(&format!("Finished compress {} after {}", "images".cyan().bold(), elapsed_time), log_func.clone());
            return Err(String::from("original path has no files !"));
        }

        // 删除目录文件
        let dest_dir = &self.destination_path;
        // 判断 origin 和 dest 目录是否相等, 如果不相等则清空 dest 目录
        if dest_dir != &self.original_path {
            log(&format!("clear dest dir: {}", dest_dir.as_path().to_string_lossy().to_string().red().bold()), log_func.clone());
            // 不存在则创建, 存在则清空
            match dir::create(dest_dir, true) {
                Ok(_) => {}
                Err(err) => {
                    // let err_msg = format!("operate dest dir: {}, error", dest_dir.as_path().to_string_lossy().to_string());
                    let msg = format!("operate dest dir: {} error: {:#?}", dest_dir.as_path().to_string_lossy().to_string().magenta().bold(), err);
                    log(&msg, log_func.clone());
                    return Err(msg.to_string())
                }
            }
        }

        // 设置队列
        let queue = Arc::new(SegQueue::new());
        for i in files {
            queue.push(i);
        }

        let mut handles = Vec::new();

        for _ in 0..self.thread_count {
            let arc_queue = Arc::clone(&queue);
            let arc_args = Arc::new(Compressor {
                factor: self.factor.clone(),
                original_path: self.original_path.clone(),
                destination_path: self.destination_path.clone(),
                thread_count: self.thread_count.clone(),
                image_size: self.image_size,
            });

            let log_func_cloned = log_func.clone();
            let handle = thread::spawn(move || {
                process(arc_queue, &*arc_args.clone(), log_func_cloned.clone());
            });

            handles.push(handle);
        }

        for h in handles {
            h.join().unwrap();
        }

        log(&format!("Compress complete {} !", "success".cyan().bold()), log_func.clone());
        let elapsed_time = format!("{:.2?}", start_time.elapsed()).magenta().bold();
        log(&format!("Finished compress {} after {}", "images".cyan().bold(), elapsed_time), log_func.clone());

        Ok(true)
    }
}

fn process<F>(queue: Arc<SegQueue<CompressorFile>>, compressor: &Compressor, log_func: Arc<Mutex<F>>)
where
    F: FnMut(&str)
{
    while !queue.is_empty() {
        match queue.pop() {
            None => break,
            Some(file) => {
                let file_path = PathBuf::from(&file.path);
                let new_dest_path = &compressor.destination_path.join(&file.relative_path);

                // 获取临时文件
                let file_stem = &file.file_stem;
                let temp_file_name = String::from(file_stem) + "_tmp." + &file.extension;
                let tmp_relative_path = &file.relative_path.replace(&file.file_name, &temp_file_name);
                let new_dest_tmp_file_path = &compressor.destination_path.join(tmp_relative_path);

                compress(&file_path, &new_dest_path, &new_dest_tmp_file_path, &file, compressor, log_func.clone());
            }
        }
    }
}

/// 转换
fn compress<F>(origin_file_path: &PathBuf, dest_file_path: &PathBuf, dest_tmp_file_path: &PathBuf, file: &CompressorFile, compressor: &Compressor, log_func: Arc<Mutex<F>>) -> bool
    where
        F: FnMut(&str)
{
    let mut factor = compressor.factor.clone();
    let file_relative_path = &file.relative_path;
    let extension = &file.extension;

    if !(factor.quality >= 0.0 && factor.quality <= 100.0) {
        log(&format!("please check factor quality: {}", factor.quality()), log_func.clone());
        return false;
    }

    if !(factor.size_ratio >= 0.0 && factor.size_ratio <= 1.0) {
        log(&format!("please check factor size_ratio: {}", factor.size_ratio()), log_func.clone());
        return false;
    }

    if factor.quality == 0.0 {
        factor.quality = factor.get_default_quality();
        log(&format!("quality is zero, use default quality: {}", factor.quality()), log_func.clone());
    }

    if factor.size_ratio == 0.0 {
        factor.size_ratio = factor.get_default_size_ratio();
        log(&format!("size_ratio is zero, use default size_ratio: {}", factor.size_ratio()), log_func.clone());
    }

    let parent = match dest_file_path.parent() {
        Some(parent) => Some(parent),
        None => None,
    };

    if parent.is_none() {
        log(&format!("get file path: {} parent error!", dest_file_path.as_path().to_string_lossy().to_string()), log_func.clone());
        return false;
    }

    let success = match fs::create_dir_all(parent.unwrap()) {
        Ok(_) => true,
        Err(err) => {
            log(&format!("create file path: {} error: {}", dest_file_path.as_path().to_string_lossy().to_string(), err), log_func.clone());
            false
        }
    };

    if !success {
        return false;
    }

    // println!("generate image path: {}", dest_file_path.as_path().to_string_lossy().to_string());
    // println!("generate tmp image path: {}", dest_tmp_file_path.as_path().to_string_lossy().to_string());

    let is_same_dir = &compressor.original_path.as_path().to_string_lossy().to_string() == &compressor.destination_path.as_path().to_string_lossy().to_string();
    if extension == "png" {
        Img::compress_png(origin_file_path, factor.quality(), dest_file_path, dest_tmp_file_path, file, is_same_dir, log_func.clone());
    } else if extension == "gif" {
        Img::compress_gif(origin_file_path, dest_file_path, dest_tmp_file_path, file, is_same_dir, log_func.clone());
    } else {
        let img_resize = Img::resize(origin_file_path, factor.size_ratio(), log_func.clone());
        if img_resize.is_none() {
            return false;
        }

        let img_resize = img_resize.unwrap();
        Img::compress_jpg(img_resize, factor.quality(), dest_file_path, file_relative_path, log_func.clone());
    }

    return true;
}

/// 记录日志
pub fn log<F>(msg: &str, log_func: Arc<Mutex<F>>)
    where
        F: FnMut(&str),
{
    println!("{}", msg);
    let mut log_func = log_func.lock().unwrap();
    (*log_func)(msg);
}
