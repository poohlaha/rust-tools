//! 图片压缩

use crate::factor::Factor;
use crate::img::Img;
use crossbeam_queue::SegQueue;
use fs_extra::dir;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::sync::Arc;
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
    pub fn compress(self) -> Result<bool, String> {
        if !self.original_path.exists() {
            println!("original path: {} is not exists", self.original_path.as_path().to_string_lossy().to_string().magenta().bold());
            return Err(String::from("original path is Empty !"));
        }

        println!("Starting compress {} ...", "images".cyan().bold());
        let start_time = Instant::now();

        let mut files: Vec<CompressorFile> = Vec::new();
        self.get_origin_file_list(&self.original_path, &mut files);
        println!("total file count: {}", files.len().to_string().cyan().bold());

        if files.len() == 0 {
            let elapsed_time = format!("{:.2?}", start_time.elapsed()).magenta().bold();
            println!("Finished compress {} after {}", "images".cyan().bold(), elapsed_time);
            return Err(String::from("original path has no files !"));
        }

        // 删除目录文件
        let dest_dir = &self.destination_path;
        // 判断 origin 和 dest 目录是否相等, 如果不相等则清空 dest 目录
        if dest_dir != &self.original_path {
            println!("clear dest dir: {}", dest_dir.as_path().to_string_lossy().to_string().red().bold());
            // 不存在则创建, 存在则清空
            match dir::create(dest_dir, true) {
                Ok(_) => {}
                Err(err) => {
                    // let err_msg = format!("operate dest dir: {}, error", dest_dir.as_path().to_string_lossy().to_string());
                    println!("operate dest dir: {} error: {:#?}", dest_dir.as_path().to_string_lossy().to_string().magenta().bold(), err);
                    return Err(format!("operate dest dir: {} error: {}", dest_dir.as_path().to_string_lossy().to_string(), err.to_string()));
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

            let handle = thread::spawn(move || {
                process(arc_queue, &*arc_args.clone());
            });

            handles.push(handle);
        }

        for h in handles {
            h.join().unwrap();
        }

        println!("Compress complete {} !", "success".cyan().bold());
        let elapsed_time = format!("{:.2?}", start_time.elapsed()).magenta().bold();
        println!("Finished compress {} after {}", "images".cyan().bold(), elapsed_time);

        Ok(true)
    }
}

fn process(queue: Arc<SegQueue<CompressorFile>>, compressor: &Compressor) {
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

                compress(&file_path, &new_dest_path, &new_dest_tmp_file_path, &file, compressor);
            }
        }
    }
}

/// 转换
fn compress(origin_file_path: &PathBuf, dest_file_path: &PathBuf, dest_tmp_file_path: &PathBuf, file: &CompressorFile, compressor: &Compressor) -> bool {
    let factor = &compressor.factor;
    let file_relative_path = &file.relative_path;
    let extension = &file.extension;

    let parent = match dest_file_path.parent() {
        Some(parent) => Some(parent),
        None => None,
    };

    if parent.is_none() {
        println!("get file path: {} parent error!", dest_file_path.as_path().to_string_lossy().to_string());
        return false;
    }

    let success = match fs::create_dir_all(parent.unwrap()) {
        Ok(_) => true,
        Err(err) => {
            println!("create file path: {} error: {}", dest_file_path.as_path().to_string_lossy().to_string(), err);
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
        Img::compress_png(origin_file_path, factor.quality(), dest_file_path, dest_tmp_file_path, file, is_same_dir);
    } else if extension == "gif" {
        Img::compress_gif(origin_file_path, dest_file_path, dest_tmp_file_path, file, is_same_dir);
    } else {
        let img_resize = Img::resize(origin_file_path, factor.size_ratio());
        if img_resize.is_none() {
            return false;
        }

        let img_resize = img_resize.unwrap();
        Img::compress_jpg(img_resize, factor.quality(), dest_file_path, file_relative_path);
    }

    return true;
}
