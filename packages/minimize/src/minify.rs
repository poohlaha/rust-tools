//! css/html/js 文件压缩

use std::ffi::OsStr;
use std::{fs, io};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use colored::Colorize;
use glob::{glob_with, MatchOptions};
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::targets::{Browsers, Targets};
use minify_html::{minify, Cfg};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use serde::{Deserialize, Serialize};
use crate::ecma::EcmaMinifier;

pub struct Minimize;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Args {
    pub dir: String, // 目录地址
    pub excludes: Vec<String>,

    #[serde(rename = "validateJs")]
    pub validate_js: bool, // 是否进行 JS 检查, 如果要检查就要使用 swc 的包, 需要牺牲性能

    #[serde(rename = "optimizationCss")]
    pub optimization_css: bool // 是否做 CSS 优化, 如果要优化，会合并多个属性, 并做代码简化
}

const DEFAULT_EXCLUDES: [&str; 8] = [
    "**/*.min.js",
    "**/*.min.css",
    "**/*.umd.js",
    "**/*.common.js",
    "**/*.esm.js",
    "**/*.amd.js",
    "**/*.iife.js",
    "**/*.cjs.js"
];

// 默认后缀
const DEFAULT_SUFFIX: [&str;4] = [
    "html",
    "js",
    "css",
    "json"
];

impl Minimize {

    pub fn exec(args: &Args) -> bool {
        // dir
        let dir = Path::new(&args.dir);
        println!("minimize dir: {:#?}", dir);

        let mut dir_str = dir.to_string_lossy().to_string();
        println!("minimize relative path: {:?}", &dir_str);
        if !dir.exists() {
            println!("minimize dir failed, `{:#?}` not exists !", dir);
            return false;
        }

        let dir = dir.join("**/*");
        dir_str = dir.as_path().to_string_lossy().to_string();

        // excludes
        let excludes: Vec<String> = Self::get_excludes(args.excludes.clone());
        println!("minimize excludes: {:#?}", excludes);

        let options = MatchOptions {
            case_sensitive: false,
            require_literal_separator: false,
            require_literal_leading_dot: false,
        };

        let entries = glob_with(&dir_str, options.clone());
        let paths = match entries {
            Ok(entries) => {
                let mut paths: Vec<PathBuf> = Vec::new();
                for entry in entries {
                    if let Ok(path) = entry {
                        let exclude_path_str = path.as_path().to_string_lossy().to_string();
                        if excludes.iter().any(|pattern| {
                            glob::Pattern::new(pattern).map(|pat| {
                                pat.matches_path_with(&path.as_path(), options.clone())
                            }).unwrap_or(false)
                        }) {
                            println!("exclude path: `{}`!", &exclude_path_str);
                            continue;
                        }

                        let file_extension = path.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");
                        if path.is_file() && DEFAULT_SUFFIX.contains(&file_extension) {
                            paths.push(path.clone())
                        }
                    }
                }

                paths
            }
            Err(err) => {
                println!("minimize error: {:#?} !", err);
                Vec::new()
            }
        };

        if paths.is_empty() {
            println!("can not found files !");
            return false;
        }

        // 开启并行任务
        Self::par(paths, args);
        return true;
    }

    // 开启并行任务
    fn par(paths: Vec<PathBuf>, args: &Args) {
        println!("found files count: {}", paths.len().to_string().magenta().bold());
        let pool = ThreadPoolBuilder::new()
            .num_threads(4)
            .stack_size(20 * 1024 * 1024)
            .build().unwrap();

        pool.install(|| {
            paths.par_iter().for_each(|path| {
                let result = Self::minify_file(path, args.validate_js, args.optimization_css);
                match result {
                    Ok(_) => {
                        let path_str = path.to_string_lossy().to_string();
                        println!("{} Minimize File: {}", "✔".green().bold(), &path_str);
                    }
                    Err(err) => {
                        println!("minimize path: `{:?}` error: {:#?} !", &path, err);
                    }
                }
            });
        });
    }

    // 压缩代码
    fn minify_file(path: &PathBuf, validate_js: bool, optimization_css: bool) -> io::Result<()> {
        let file_extension = path.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");

        let mut file = fs::File::open(path)?;
        let mut code = String::new();
        file.read_to_string(&mut code)?;

        let mut minified = Vec::new();
        if file_extension == DEFAULT_SUFFIX[0] { // html
            let mut cfg = Cfg::new();
            cfg.remove_bangs = false;
            cfg.remove_processing_instructions = false;
            cfg.preserve_chevron_percent_template_syntax = true;
            cfg.preserve_brace_template_syntax = true;
            cfg.keep_comments = false;
            cfg.minify_css = true;
            cfg.minify_js = true;
            minified = minify(code.as_bytes(), &cfg);
        } else if file_extension == DEFAULT_SUFFIX[1] { // js
            if validate_js {
                minified = EcmaMinifier::exec(path)
            } else {
                minified = minifier::js::minify(&code).to_string().into_bytes();
            }
        }  else if file_extension == DEFAULT_SUFFIX[2] { // css
            // 此处使用 minifier::css::minify 会把中间的空格去除
            /*
            minified = match minifier::css::minify(&code) {
                Ok(str) => str.to_string().into_bytes(),
                Err(err) => {
                    println!("minimize path: `{:?}` error: {:#?} !", &path, err);
                    Vec::new()
                }
            }
             */
            minified = Self::minify_css(path, &code, optimization_css);
        }  else if file_extension == DEFAULT_SUFFIX[3] { // json
            minified = minifier::json::minify(&code).to_string().into_bytes();
        }

        if minified.is_empty() {
            return Ok(())
        }

        let mut file = fs::File::create(path)?;
        file.write_all(&minified)?;
        file.sync_all().unwrap(); // 写入磁盘
        drop(file); // 自动关闭文件
        Ok(())
    }


    fn get_excludes(excludes: Vec<String>) -> Vec<String> {
        let mut default_excludes: Vec<String> = DEFAULT_EXCLUDES.iter().map(|&s| s.to_string()).collect();
        default_excludes.extend(excludes);
        return default_excludes
    }

    /// 压缩 css
    fn minify_css(path: &PathBuf, code: &str, optimization_css: bool) -> Vec<u8> {
        let get_result = |stylesheet: StyleSheet| {
            let result = stylesheet.to_css(PrinterOptions {
                minify: true,
                ..PrinterOptions::default()
            });
            return match result {
                Ok(result) => {
                    result.code.into_bytes()
                }
                Err(err) => {
                    println!("minimize path: `{:?}` error: {:#?} !", &path, err);
                    Vec::new()
                }
            }
        };

        let stylesheet = StyleSheet::parse(&code, ParserOptions::default());
        return match stylesheet {
            Ok(mut stylesheet) => {
                let mut options = lightningcss::stylesheet::MinifyOptions::default();
                options.targets = Targets {
                    browsers: Some(Browsers {
                        ios_saf: Some(8),
                        safari: Some(8),
                        ..Default::default()
                    }),
                    include: Default::default(),
                    exclude: Default::default(),
                };

                if optimization_css {
                    return match stylesheet.minify(options) {
                        Ok(_) => {
                            get_result(stylesheet)
                        },
                        Err(err) => {
                            println!("minimize path: `{:?}` error: {:#?} !", &path, err);
                            Vec::new()
                        }
                    }

                } else {
                    return get_result(stylesheet)
                }
            },
            Err(err) => {
                println!("minimize path: `{:?}` error: {:#?} !", &path, err);
                Vec::new()
            }
        }
    }
}