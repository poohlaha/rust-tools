//! 图片压缩

use std::ffi::OsStr;
use std::{fs, thread};
use std::io::{Write};
use std::path::{PathBuf};
use std::sync::{Arc};
use fs::File;
use std::time::Instant;
use crossbeam_queue::SegQueue;
use crate::factor::Factor;
use fs_extra::dir;
use crate::img::Img;
use colored::*;
use crate::LOGGER_PREFIX;

pub struct Compressor {
    factor: Factor,
    original_path: PathBuf,
    destination_path: PathBuf,
    thread_count: u32,
    need_convert_format: bool,
    image_size: u64
}

#[derive(Debug)]
pub struct CompressorArgs {
    pub factor: Option<Factor>,
    pub origin: String,
    pub dest: String,
    pub thread_count: Option<u32>,
    pub need_convert_format: bool, // 是否转换为原来对应的格式, 如果转换会导致图片过大, 默认是转成 jpeg, 只是改了后缀
    pub image_size: u64, // 要压缩的图片最小值, 默认为 kb
}

struct CompressorFile {
    #[allow(dead_code)]
    file_name: String, // 文件名
    extension: String, // 后缀
    path: String, // 全路径
    relative_path: String, // 相对路径
}

const FILE_LIST: [&str; 3] = ["jpg", "jpeg", "png"];
impl Compressor {

    pub fn new(args: CompressorArgs) -> Compressor {
        let factor = args.factor;
        let thread_count = args.thread_count;

        Compressor {
            factor: if factor.is_none() { Factor::default() } else { factor.clone().unwrap() },
            original_path: PathBuf::from(args.origin),
            destination_path: PathBuf::from(args.dest),
            thread_count: if factor.is_none() { 1 } else { thread_count.unwrap() },
            need_convert_format: args.need_convert_format,
            image_size: args.image_size
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
                let extension = path.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
                let size = fs::metadata(&path).unwrap().len();
                if self.image_size == 0 {
                    if FILE_LIST.contains(&extension) {
                        files.push(CompressorFile {
                            extension: extension.to_string(),
                            path: path.as_path().to_string_lossy().to_string(),
                            file_name: path.file_name().unwrap().to_string_lossy().to_string(),
                            relative_path: relative_path.to_string()
                        })
                    }

                    continue
                }

                if size > self.image_size * 1024 {
                    if FILE_LIST.contains(&extension) {
                        files.push(CompressorFile {
                            extension: extension.to_string(),
                            path: path.as_path().to_string_lossy().to_string(),
                            file_name: path.file_name().unwrap().to_string_lossy().to_string(),
                            relative_path: relative_path.to_string()
                        })
                    }
                }
            }
        }
    }

    /// compress
    pub fn compress(self) {
        if !self.original_path.exists() {
            println!("{} original path: {} is not exists", LOGGER_PREFIX.cyan().bold(), self.original_path.as_path().to_string_lossy().to_string().magenta().bold());
            return;
        }

        println!("{} Starting compress {} ...", LOGGER_PREFIX.cyan().bold(), "images".cyan().bold());
        let start_time = Instant::now();

        let mut files: Vec<CompressorFile> = Vec::new();
        self.get_origin_file_list(&self.original_path, &mut files);
        println!("{} total file count: {}", LOGGER_PREFIX.cyan().bold(), files.len().to_string().cyan().bold());

        if files.len() == 0 {
            let elapsed_time = format!("{:.2?}", start_time.elapsed()).magenta().bold();
            println!("{} Finished compress {} after {}", LOGGER_PREFIX.cyan().bold(), "images".cyan().bold(), elapsed_time);
            return;
        }

        // 删除目录文件
        let dest_dir = &self.destination_path;
        // 判断 origin 和 dest 目录是否相等, 如果不相等则清空 dest 目录
        if dest_dir != &self.original_path {
            println!("{} clear dest dir: {}", LOGGER_PREFIX.cyan().bold(), dest_dir.as_path().to_string_lossy().to_string().red().bold());
            // 不存在则创建, 存在则清空
            match dir::create(dest_dir, true) {
                Ok(_) => {}
                Err(err) => {
                    let err_msg = format!("operate dest dir: {}, error", dest_dir.as_path().to_string_lossy().to_string());
                    println!("{} operate dest dir: {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), dest_dir.as_path().to_string_lossy().to_string().magenta().bold(), err);
                    panic!("{}", err_msg)
                }
            }
        }

        // 设置队列
        let queue = Arc::new(SegQueue::new());
        for i in files {
            queue.push(i);
        }

        let mut handles = Vec::new();

        for _ in 0 .. self.thread_count {
            let arc_queue = Arc::clone(&queue);
            let arc_args = Arc::new(Compressor {
                factor: self.factor.clone(),
                original_path: self.original_path.clone(),
                destination_path: self.destination_path.clone(),
                thread_count: self.thread_count.clone(),
                need_convert_format: self.need_convert_format,
                image_size: self.image_size
            });

            let handle = thread::spawn(move || {
                process(arc_queue, &*arc_args.clone());
            });

            handles.push(handle);
        }

        for h in handles {
            h.join().unwrap();
        }

        println!("{} Compress complete {} !", LOGGER_PREFIX.cyan().bold(), "success".cyan().bold());
        let elapsed_time = format!("{:.2?}", start_time.elapsed()).magenta().bold();
        println!("{} Finished compress {} after {}", LOGGER_PREFIX.cyan().bold(), "images".cyan().bold(), elapsed_time);
    }

}

fn process(queue: Arc<SegQueue<CompressorFile>>, compressor: &Compressor) {
    while !queue.is_empty() {
        match queue.pop() {
            None => break,
            Some(file) => {
                let file_path = PathBuf::from(&file.path);
                let new_dest_path = &compressor.destination_path.join(&file.relative_path);
                compress(&file_path, &new_dest_path, &file, compressor);
            }
        }
    }
}

/// 转换
fn compress(origin_file_path: &PathBuf, dest_file_path: &PathBuf, file: &CompressorFile, compressor: &Compressor) -> bool {
    let factor = &compressor.factor;
    let file_relative_path = &file.relative_path;
    let extension = &file.extension;
    let need_convert_format = compressor.need_convert_format;
    let img = Img::resize(origin_file_path, factor.size_ratio());
    if img.is_none() {
        return false;
    }

    let img = img.unwrap();
    let compressed_img_data = Img::compress(img, factor.quality());
    if compressed_img_data.is_none() {
        return false;
    }

    if compressed_img_data.is_none() {
        return false;
    }

    let compressed_img_data = compressed_img_data.unwrap();

    let parent = match dest_file_path.parent()  {
        Some(parent) => Some(parent),
        None => None
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

    let output_file = match File::create(dest_file_path.clone()) {
        Ok(file) => Some(file),
        Err(err) => {
            println!("{} create file path: {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), dest_file_path.as_path().to_string_lossy().to_string(), err);
            None
        }
    };

    if output_file.is_none() {
        return false;
    }

    let mut output_file = output_file.unwrap();
    if need_convert_format {
        if extension == "png" {
            let img = image::load_from_memory(&compressed_img_data).unwrap();
            return match img.save_with_format(dest_file_path.clone().as_path(), image::ImageFormat::Png) {
                Ok(_) => true,
                Err(err) => {
                    println!("{} convert to origin format error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                    false
                }
            }
        }
    }

    return match output_file.write_all(&compressed_img_data) {
        Ok(_) => {
            println!("{} compress file: {} success !", LOGGER_PREFIX.cyan().bold(), file_relative_path.red().bold());
            true
        }
        Err(err) => {
            println!("{} compress file: {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), file_relative_path.red().bold(), err);
            false
        }
    };
}