//! 图片操作

use crate::compressor::{log, CompressorFile};
use colored::Colorize;
use image::imageops::FilterType;
use imagequant::Attributes;
use lodepng::decode32_file;
use mozjpeg::{ColorSpace, Compress, ScanMode};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Img;

#[derive(Clone)]
pub struct ImgResized {
    rgb8: Vec<u8>,
    pub(crate) width: usize,
    pub(crate) height: usize,
}

impl Img {
    pub fn resize<F>(file_path: &PathBuf, resize_ratio: f32, log_func: Arc<Mutex<F>>) -> Option<ImgResized>
    where
        F: FnMut(&str),
    {
        let img = match image::open(file_path) {
            Ok(img) => Some(img),
            Err(err) => {
                log(&format!("open image: {} error: {:#?}", file_path.as_path().to_string_lossy().to_string().red().bold(), err), log_func.clone());
                None
            }
        };

        if img.is_none() {
            return None;
        }

        let img = img.unwrap();
        let width = img.width() as usize;
        let height = img.height() as usize;

        let width = width as f32 * resize_ratio;
        let height = height as f32 * resize_ratio;

        let resized_img = img.resize(width as u32, height as u32, FilterType::Triangle);
        let resized_width = resized_img.width() as usize;
        let resized_height = resized_img.height() as usize;

        return Some(ImgResized {
            rgb8: resized_img.into_rgb8().into_vec(),
            width: resized_width,
            height: resized_height,
        });
    }

    /// 压缩 jpg
    pub fn compress_jpg<F>(img_resized: ImgResized, quality: f32, dest_file_path: &PathBuf, file_relative_path: &str, log_func: Arc<Mutex<F>>) -> bool
    where
        F: FnMut(&str),
    {
        let target_width = img_resized.width;
        let target_height = img_resized.height;
        let resized_img_data = img_resized.rgb8;

        let mut comp = Compress::new(ColorSpace::JCS_RGB); // 创建了一个 JPEG 压缩器, 指定了颜色空间为 RGB
        comp.set_scan_optimization_mode(ScanMode::Auto); // 设置压缩器的扫描优化模式为自动模式, 扫描优化是 JPEG 压缩中的一种技术，用于提高压缩效率
        comp.set_quality(quality); // 设置 JPEG 图像的质量
        comp.set_size(target_width, target_height); // 设置输出图像的尺寸
        comp.set_mem_dest(); // 设置输出目标为内存, 压缩后的 JPEG 数据将被写入内存而不是文件
        comp.set_optimize_scans(true); // 启用扫描优化
        comp.start_compress();

        let mut line = 0;
        loop {
            if line > target_height - 1 {
                break;
            }
            comp.write_scanlines(&resized_img_data[line * target_width * 3..(line + 1) * target_width * 3]);
            line += 1;
        }
        comp.finish_compress();

        let compressed = match comp.data_to_vec() {
            Ok(compressed) => Some(compressed),
            Err(_) => {
                log("compress image error !", log_func.clone());
                None
            }
        };

        if compressed.is_none() {
            return false;
        }

        let compressed = compressed.unwrap();

        let output_file = match File::create(dest_file_path.clone()) {
            Ok(file) => Some(file),
            Err(err) => {
                log(&format!("create file path: {} error: {:#?}", dest_file_path.as_path().to_string_lossy().to_string(), err), log_func.clone());
                None
            }
        };

        if output_file.is_none() {
            return false;
        }

        let mut output_file = output_file.unwrap();

        let flag = match output_file.write_all(&compressed) {
            Ok(_) => {
                log(&format!("compress `JPG` file: {} success !", file_relative_path.cyan().bold()), log_func.clone());
                true
            }
            Err(err) => {
                log(&format!("compress `JPG` file: {} error: {:#?}", file_relative_path.red().bold(), err), log_func.clone());
                false
            }
        };

        return flag;
    }

    /// 压缩 png
    pub fn compress_png<F>(file_path: &PathBuf, quality: f32, dest_file_path: &PathBuf, dest_tmp_file_path: &PathBuf, file: &CompressorFile, is_same_dir: bool, log_func: Arc<Mutex<F>>) -> bool
    where
        F: FnMut(&str),
    {
        let bitmap = match decode32_file(file_path) {
            Ok(bitmap) => Some(bitmap),
            Err(err) => {
                log(&format!("open image: {} error: {:#?}", file_path.as_path().to_string_lossy().to_string().red().bold(), err), log_func.clone());
                None
            }
        };

        if bitmap.is_none() {
            return false;
        }

        let bitmap = bitmap.unwrap();
        let width = bitmap.width;
        let height = bitmap.height;

        let mut attribute = Attributes::new();
        attribute.set_speed(10).unwrap(); // 设置压缩速度，可以根据需要进行调整
        attribute.set_quality(quality as u8, 99).unwrap(); // 设置品质最小值和最大值, 默认为 0 - 100

        let img = match attribute.new_image(&*bitmap.buffer, width, height, 0.0) {
            Ok(img) => Some(img),
            Err(err) => {
                log(&format!("handle `PNG` image: {} error: {:#?}", &file.relative_path.red().bold(), err), log_func.clone());
                None
            }
        };

        if img.is_none() {
            return false;
        }

        let mut img = img.unwrap();
        // 执行图像量化
        let result = match attribute.quantize(&mut img) {
            Ok(result) => Some(result),
            Err(err) => {
                log(&format!("handle `PNG` image: {} error: {:#?}", &file.relative_path.red().bold(), err), log_func.clone());
                None
            }
        };

        if result.is_none() {
            return false;
        }

        let mut result = result.unwrap();
        // 为后续重新映射启用抖动
        let flag = match result.set_dithering_level(1.0) {
            Ok(_) => true,
            Err(err) => {
                log(&format!("handle `PNG` image: {} error: {:#?}", &file.relative_path.red().bold(), err), log_func.clone());
                false
            }
        };

        if !flag {
            return false;
        }

        let value = match result.remapped(&mut img) {
            Ok(value) => Some(value),
            Err(err) => {
                log(&format!("handle `PNG` image: {} error: {:#?}", &file.relative_path.red().bold(), err), log_func.clone());
                None
            }
        };

        if value.is_none() {
            return false;
        }

        let (palette, pixels) = value.unwrap();
        // 遍历像素数据，将每个索引映射到调色板上的RGB颜色，并将RGBA像素添加到rgba_pixels中
        let mut rgba_pixels: Vec<u8> = Vec::new();
        for &index in &pixels {
            if let Some(color) = palette.get(index as usize) {
                // 添加RGB颜色值
                rgba_pixels.push(color.r);
                rgba_pixels.push(color.g);
                rgba_pixels.push(color.b);
                // 添加Alpha通道值，可以根据需要设置
                rgba_pixels.push(color.a); // 255表示完全不透明
            } else {
                // 如果无法映射索引到颜色，可以选择默认颜色值
                rgba_pixels.push(0); // Red
                rgba_pixels.push(0); // Green
                rgba_pixels.push(0); // Blue
                rgba_pixels.push(255); // Alpha
            }
        }

        // 创建一个新的PNG图像文件
        let flag = match lodepng::encode_file(dest_tmp_file_path, &rgba_pixels, width, height, lodepng::ColorType::RGBA, 8) {
            Ok(_) => true,
            Err(err) => {
                log(&format!("regenerate `PNG` image: {} error: {:#?}", &file.relative_path.red().bold(), err), log_func.clone());
                false
            }
        };

        if !flag {
            return false;
        }

        return Img::validate_image(dest_tmp_file_path, dest_file_path, file, is_same_dir, "PNG", log_func.clone());
    }

    /// 压缩 gif
    pub fn compress_gif<F>(file_path: &PathBuf, dest_file_path: &PathBuf, dest_tmp_file_path: &PathBuf, file: &CompressorFile, is_same_dir: bool, log_func: Arc<Mutex<F>>) -> bool
    where
        F: FnMut(&str),
    {
        let img = match File::open(file_path) {
            Ok(img) => Some(img),
            Err(err) => {
                log(&format!("open `GIF` image: {} error: {:#?}", file_path.as_path().to_string_lossy().to_string().red().bold(), err), log_func.clone());
                None
            }
        };

        if img.is_none() {
            return false;
        }

        let img = img.unwrap();
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::Indexed);

        let decoder = match options.read_info(img) {
            Ok(decoder) => Some(decoder),
            Err(err) => {
                log(&format!("regenerate `GIF` image: {} error: {:#?}", &file.relative_path.red().bold(), err), log_func.clone());
                None
            }
        };

        if decoder.is_none() {
            return false;
        }

        let mut decoder = decoder.unwrap();
        let screen_width = decoder.width();
        let screen_height = decoder.height();
        let global_pal = decoder.global_palette().unwrap_or_default().to_vec();

        let mut output_file = File::create(dest_tmp_file_path).unwrap();
        let encoder = match gif::Encoder::new(&mut output_file, screen_width, screen_height, &global_pal) {
            Ok(encoder) => Some(encoder),
            Err(err) => {
                log(&format!("regenerate `GIF` image: {} error: {:#?}", &file.relative_path.red().bold(), err), log_func.clone());
                None
            }
        };

        if encoder.is_none() {
            return false;
        }

        let mut encoder = encoder.unwrap();
        let mut frame_number = 1;
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            // 减少帧数（每隔一帧写一个帧）
            if frame_number % 2 == 0 {
                frame_number += 1;
                continue;
            }

            let mut new_frame = gif::Frame::default();
            new_frame.delay = frame.delay + 1; // 设置帧间隔（以1/100秒为单位），根据需要调整
            new_frame.width = frame.width;
            new_frame.height = frame.height;
            new_frame.dispose = frame.dispose;
            new_frame.transparent = frame.transparent;
            new_frame.needs_user_input = frame.needs_user_input;
            new_frame.top = frame.top;
            new_frame.left = frame.left;
            new_frame.interlaced = frame.interlaced;
            new_frame.palette = frame.palette.clone();
            new_frame.buffer = frame.buffer.clone();

            let success = match encoder.write_frame(&new_frame) {
                Ok(_) => true,
                Err(err) => {
                    log(&format!("regenerate `GIF` image: {} error: {:#?}", &file.relative_path.red().bold(), err), log_func.clone());
                    false
                }
            };

            if !success {
                return false;
            }

            frame_number += 1;
        }

        return Img::validate_image(dest_tmp_file_path, dest_file_path, file, is_same_dir, "GIF", log_func.clone());
    }

    /// 校验图片, 判断压缩后图片是不是大于原图片, 如果大于, 则取消压缩
    fn validate_image<F>(dest_tmp_file_path: &PathBuf, dest_file_path: &PathBuf, file: &CompressorFile, is_same_dir: bool, name: &str, log_func: Arc<Mutex<F>>) -> bool
    where
        F: FnMut(&str),
    {
        // 判断压缩后图片是不是大于原图片, 如果大于, 则取消压缩
        let tmp_file_size = fs::metadata(dest_tmp_file_path).unwrap().len();
        let success;
        if tmp_file_size >= file.file_size {
            log(&format!("regenerate `{}` file size: {} bytes, big", name.cyan().bold(), tmp_file_size.to_string().red().bold()), log_func.clone());
            // 删除临时文件
            let is_ok = match fs_extra::file::remove(dest_tmp_file_path.as_path().to_string_lossy().to_string()) {
                Ok(_) => {
                    log(&format!("compress `{}` file: {} success !", name.cyan().bold(), &file.relative_path.cyan().bold()), log_func.clone());
                    true
                }
                Err(err) => {
                    log(&format!("regenerate `{}` image: {} error: {:#?}", name.cyan().bold(), &file.relative_path.red().bold(), err), log_func.clone());
                    false
                }
            };

            if !is_ok {
                return false;
            }

            // 不是同一目录, 需要拷贝原来文件到目录
            if !is_same_dir {
                success = match fs_extra::file::copy(&file.path, dest_file_path.as_path().to_string_lossy().to_string(), &fs_extra::file::CopyOptions::new()) {
                    Ok(_) => {
                        log(&format!("compress `{}` file: {} success !", name.cyan().bold(), &file.relative_path.cyan().bold()), log_func.clone());
                        true
                    }
                    Err(err) => {
                        log(&format!("regenerate `{}` image: {} error: {:#?}", name.cyan().bold(), &file.relative_path.red().bold(), err), log_func.clone());
                        false
                    }
                };
            } else {
                success = true;
            }
        } else {
            // 移动文件
            let mut options = fs_extra::file::CopyOptions::new();
            options = options.overwrite(true);
            success = match fs_extra::file::move_file(dest_tmp_file_path.as_path().to_string_lossy().to_string(), dest_file_path.as_path().to_string_lossy().to_string(), &options) {
                Ok(_) => {
                    log(&format!("compress `{}` file: {} success !", name.cyan().bold(), &file.relative_path.cyan().bold()), log_func.clone());
                    true
                }
                Err(err) => {
                    log(&format!("regenerate `{}` image: {} error: {:#?}", name.cyan().bold(), &file.relative_path.red().bold(), err), log_func.clone());
                    false
                }
            };
        }

        return success;
    }
}
