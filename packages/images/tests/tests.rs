//! 测试

use images_compressor::compressor::{Compressor, CompressorArgs};
use images_compressor::factor::Factor;

#[test]
fn test_image_compressor() {
    let factor = Factor {
        quality: 80.0,   // 品质: 0 - 100
        size_ratio: 0.8, // // 压缩比例: 0 - 1
    };

    let args = CompressorArgs {
        factor: Some(factor),
        origin: "/usr/local/images/outputs".to_string(),
        dest: "/usr/local/images/outputs".to_string(),
        thread_count: None,
        image_size: 0,
    };

    let success = Compressor::new(args).compress(|str| {}).unwrap();
    assert_eq!(success, true)
}
