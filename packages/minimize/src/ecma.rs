//! 使用 swc 的 swc_ecma_minifier 进行 js 压缩、检查等

use crate::minify::Minimize;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use swc_common::sync::Lrc;
use swc_common::{FilePathMapping, SourceMap};
use swc_ecma_codegen::text_writer::{omit_trailing_semi, JsWriter};
use swc_ecma_minifier::option::{ExtraOptions, MangleOptions, MinifyOptions};
use swc_ecma_transforms_base::fixer::fixer;
use swc_ecma_transforms_base::resolver;
use swc_ecma_visit::FoldWith;

pub struct EcmaMinifier;

impl EcmaMinifier {
    pub fn exec<F>(path: &PathBuf, log_func: Arc<Mutex<F>>) -> Vec<u8>
    where
        F: FnMut(&str),
    {
        let result = EcmaMinifier::run(|cm| {
            let fm = match cm.load_file(path) {
                Ok(fm) => Some(fm),
                Err(err) => {
                    Minimize::log(&format!("Ecma Minifier load file error: {:#?}", err), log_func.clone());
                    None
                }
            };

            if fm.is_none() {
                return Err(());
            }

            let fm = fm.unwrap();
            let unresolved_mark = swc_common::Mark::new();
            let top_level_mark = swc_common::Mark::new();

            let module = swc_ecma_parser::parse_file_as_module(&fm, Default::default(), Default::default(), None, &mut vec![]);

            let program = match module.map(|module| module.fold_with(&mut resolver(unresolved_mark, top_level_mark, false))) {
                Ok(program) => Some(program),
                Err(err) => {
                    Minimize::log(&format!("Ecma Minifier error: {:#?}", err), log_func.clone());
                    None
                }
            };

            if program.is_none() {
                return Err(());
            }

            let program = program.unwrap();
            let minify_options = MinifyOptions {
                compress: Some(Default::default()),
                mangle: Some(MangleOptions {
                    top_level: Some(false),
                    keep_fn_names: true,
                    ..Default::default()
                }),
                ..Default::default()
            };

            let extra_options = ExtraOptions { unresolved_mark, top_level_mark };

            let output = swc_ecma_minifier::optimize(program.into(), cm.clone(), None, None, &minify_options, &extra_options).expect_module();

            let output = output.fold_with(&mut fixer(None));
            let code = EcmaMinifier::print(cm, &[output], true);
            Ok(code)
        });

        return match result {
            Ok(code) => code.into_bytes(),
            Err(_) => Vec::new(),
        };
    }

    fn run<F, Ret>(op: F) -> Result<Ret, ()>
    where
        F: FnOnce(Lrc<SourceMap>) -> Result<Ret, ()>,
    {
        let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
        let result = swc_common::GLOBALS.set(&swc_common::Globals::new(), || op(cm));
        match result {
            Ok(res) => Ok(res),
            Err(()) => {
                println!("Ecma Minifier error !");
                Err(())
            }
        }
    }

    fn print<N: swc_ecma_codegen::Node>(cm: Lrc<SourceMap>, nodes: &[N], minify: bool) -> String {
        let mut buf = vec![];

        {
            let mut emitter = swc_ecma_codegen::Emitter {
                cfg: swc_ecma_codegen::Config::default().with_minify(minify),
                cm: cm.clone(),
                comments: None,
                wr: omit_trailing_semi(JsWriter::new(cm, "\n", &mut buf, None)),
            };

            for n in nodes {
                n.emit_with(&mut emitter).unwrap();
            }
        }

        String::from_utf8(buf).unwrap()
    }
}
