//! 测试

use minimize::minify::{Args, Minimize};

#[test]
fn test_minimize() {
    let args = Args {
        dir: "/usr/local/www".to_string(),
        excludes: vec![],
        validate_js: false,
        optimization_css: false,
    };

    let success = Minimize::exec(&args, |str| {

    });
    assert_eq!(success, true);
}
