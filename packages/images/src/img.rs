//! 图片操作

use colored::*;
use std::path::{PathBuf};
use image::imageops::FilterType;
use mozjpeg::{ColorSpace, Compress, ScanMode};
use crate::LOGGER_PREFIX;

pub struct Img;

#[derive(Clone)]
pub struct ImgResized {
    rgb8: Vec<u8>,
    pub(crate) width: usize,
    pub(crate) height: usize
}

impl Img {

    #[allow(dead_code)]
    fn convert_to_jpg(&self, file_path: &PathBuf) -> Option<PathBuf> {
        let img = match image::open(file_path) {
            Ok(img) => Some(img),
            Err(err) => {
                println!("{} open image: {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), file_path.as_path().to_string_lossy().to_string().red().bold(), err);
                None
            }
        };

        if img.is_none() {
            return None;
        }

        let img = img.unwrap();

        let stem = file_path.file_stem().unwrap();
        let new_path = match file_path.parent() {
            Some(s) => Some(s),
            None => {
                println!("{} Cannot get parent directory!", LOGGER_PREFIX.cyan().bold());
                None
            }
        };

        if new_path.is_none() {
            return None;
        }

        let mut new_path = new_path.unwrap().join(stem);
        new_path.set_extension("jpg");
        let flag = match img.save(&new_path) {
            Ok(_) => true,
            Err(err) => {
                println!("{} save jpg image error: {:#?}", LOGGER_PREFIX.cyan().bold(), err);
                false
            }
        };

        if flag {
            return Some(new_path);
        } else {
            return None;
        }
    }

    pub fn resize(file_path: &PathBuf, resize_ratio: f32) -> Option<ImgResized> {
        let img = match image::open(file_path) {
            Ok(img) => Some(img),
            Err(err) => {
                println!("{} open image: {} error: {:#?}", LOGGER_PREFIX.cyan().bold(), file_path.as_path().to_string_lossy().to_string().red().bold(), err);
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
        })
    }

    /// 压缩
    pub fn compress(img_resized: ImgResized, quality: f32) -> Option<Vec<u8>> {
        let target_width = img_resized.width;
        let target_height = img_resized.height;
        let resized_img_data = img_resized.rgb8;

        let mut comp = Compress::new(ColorSpace::JCS_RGB);
        comp.set_scan_optimization_mode(ScanMode::Auto);
        comp.set_quality(quality);
        comp.set_size(target_width, target_height);
        comp.set_mem_dest();
        comp.set_optimize_scans(true);
        comp.start_compress();


        let mut line = 0;
        loop {
            if line > target_height - 1 {
                break;
            }
            comp.write_scanlines(
                &resized_img_data[line * target_width * 3..(line + 1) * target_width * 3],
            );
            line += 1;
        }
        comp.finish_compress();

        let compressed = match comp.data_to_vec() {
            Ok(compressed) => Some(compressed),
            Err(_) => {
                println!("{} compress image error !", LOGGER_PREFIX.cyan().bold());
                None
            }
        };

        return compressed;
    }
}