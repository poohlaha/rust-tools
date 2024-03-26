//! css/html/js 文件压缩

use crate::ecma::EcmaMinifier;
use colored::Colorize;
use glob::{glob_with, MatchOptions};
use lightningcss::printer::PrinterOptions;
use lightningcss::stylesheet::{ParserOptions, StyleSheet};
use lightningcss::targets::{Browsers, Targets};
use minify_html::{minify, Cfg};
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{fs, io};

pub struct Minimize;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Args {
    pub dir: String, // 目录地址
    pub excludes: Vec<String>,

    #[serde(rename = "validateJs")]
    pub validate_js: bool, // 是否进行 JS 检查, 如果要检查就要使用 swc 的包, 需要牺牲性能

    #[serde(rename = "optimizationCss")]
    pub optimization_css: bool, // 是否做 CSS 优化, 如果要优化，会合并多个属性, 并做代码简化
}

const DEFAULT_EXCLUDES: [&str; 8] = ["**/*.min.js", "**/*.min.css", "**/*.umd.js", "**/*.common.js", "**/*.esm.js", "**/*.amd.js", "**/*.iife.js", "**/*.cjs.js"];

// 默认后缀
const DEFAULT_SUFFIX: [&str; 4] = ["html", "js", "css", "json"];

impl Minimize {
    pub fn exec<F>(args: &Args, log_func: F) -> bool
    where
        F: FnMut(&str) + Send,
    {
        // dir
        let dir = Path::new(&args.dir);

        let log_func = Arc::new(Mutex::new(log_func));

        // 输出日志
        Self::log(&format!("minimize dir: {:#?}", dir), log_func.clone());

        let mut dir_str = dir.to_string_lossy().to_string();
        Self::log(&format!("minimize relative path: {}", dir_str), log_func.clone());

        if !dir.exists() {
            Self::log(&format!("minimize dir failed, `{:#?}` not exists !", dir), log_func.clone());
            return false;
        }

        let dir = dir.join("**/*");
        dir_str = dir.as_path().to_string_lossy().to_string();

        // excludes
        let excludes: Vec<String> = Self::get_excludes(args.excludes.clone());
        Self::log(&format!("minimize excludes: {:#?}", excludes), log_func.clone());

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
                        if excludes.iter().any(|pattern| glob::Pattern::new(pattern).map(|pat| pat.matches_path_with(&path.as_path(), options.clone())).unwrap_or(false)) {
                            Self::log(&format!("exclude path: `{}`", exclude_path_str), log_func.clone());
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
                Self::log(&format!("minimize error: {:#?}", err), log_func.clone());
                Vec::new()
            }
        };

        if paths.is_empty() {
            Self::log("can not found files !", log_func.clone());
            return false;
        }

        // 开启并行任务
        Self::par(paths, args, log_func.clone());
        return true;
    }

    // 开启并行任务
    fn par<F>(paths: Vec<PathBuf>, args: &Args, log_func: Arc<Mutex<F>>)
    where
        F: FnMut(&str) + Send,
    {
        Self::log(&format!("found files count: {}", paths.len().to_string().magenta().bold()), log_func.clone());

        let pool = ThreadPoolBuilder::new().num_threads(4).stack_size(20 * 1024 * 1024).build().unwrap();

        pool.install(|| {
            paths.par_iter().for_each(|path| {
                let result = Self::minify_file(path, args.validate_js, args.optimization_css, log_func.clone());
                match result {
                    Ok(_) => {
                        let path_str = path.to_string_lossy().to_string();
                        Self::log(&format!("{} Minimize File: {}", "✔".green().bold(), &path_str), log_func.clone());
                    }
                    Err(err) => {
                        Self::log(&format!("minimize path: `{:?}` error: {:#?}", &path, err), log_func.clone());
                    }
                }
            });
        });
    }

    // 压缩代码
    fn minify_file<F>(path: &PathBuf, validate_js: bool, optimization_css: bool, log_func: Arc<Mutex<F>>) -> io::Result<()>
    where
        F: FnMut(&str),
    {
        let file_extension = path.extension().unwrap_or(OsStr::new("")).to_str().unwrap_or("");

        let mut file = fs::File::open(path)?;
        let mut code = String::new();
        file.read_to_string(&mut code)?;

        let mut minified = Vec::new();
        if file_extension == DEFAULT_SUFFIX[0] {
            // html
            let mut cfg = Cfg::new();
            cfg.remove_bangs = false;
            cfg.remove_processing_instructions = false;
            cfg.preserve_chevron_percent_template_syntax = true;
            cfg.preserve_brace_template_syntax = true;
            cfg.keep_comments = false;
            cfg.minify_css = true;
            cfg.minify_js = true;
            minified = minify(code.as_bytes(), &cfg);
        } else if file_extension == DEFAULT_SUFFIX[1] {
            // js
            if validate_js {
                minified = EcmaMinifier::exec(path, log_func.clone())
            } else {
                minified = minifier::js::minify(&code).to_string().into_bytes();
            }
        } else if file_extension == DEFAULT_SUFFIX[2] {
            // css
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
            minified = Self::minify_css(path, &code, optimization_css, log_func.clone());
        } else if file_extension == DEFAULT_SUFFIX[3] {
            // json
            minified = minifier::json::minify(&code).to_string().into_bytes();
        }

        if minified.is_empty() {
            return Ok(());
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
        return default_excludes;
    }

    /// 压缩 css
    fn minify_css<F>(path: &PathBuf, code: &str, optimization_css: bool, log_func: Arc<Mutex<F>>) -> Vec<u8>
    where
        F: FnMut(&str),
    {
        let get_result = |stylesheet: StyleSheet| {
            let result = stylesheet.to_css(PrinterOptions { minify: true, ..PrinterOptions::default() });
            return match result {
                Ok(result) => result.code.into_bytes(),
                Err(err) => {
                    Self::log(&format!("minimize path: `{:?}` error: {:#?}", &path, err), log_func.clone());
                    Vec::new()
                }
            };
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
                        Ok(_) => get_result(stylesheet),
                        Err(err) => {
                            Self::log(&format!("minimize path: `{:?}` error: {:#?}", &path, err), log_func.clone());
                            Vec::new()
                        }
                    };
                } else {
                    return get_result(stylesheet);
                }
            }
            Err(err) => {
                Self::log(&format!("minimize path: `{:?}` error: {:#?}", &path, err), log_func.clone());
                Vec::new()
            }
        };
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
}
