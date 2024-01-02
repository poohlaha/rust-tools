# images-compressor
compress pictures, such as `png`、`jpg`、`jpeg`、`git`。

## Usage

To use `images-compressor`, first add this to your `Cargo.toml`:

```toml
[dependencies]
images-compressor = "1.0"
```

## Examples

```rust
use images_compressor::compressor::{Compressor, CompressorArgs};
use images_compressor::factor::Factor;

fn main() {
    let factor = Factor{
        quality: 80.0,  // 品质: 0 - 100
        size_ratio: 0.8, // // 压缩比例: 0 - 1
    };

    let args = CompressorArgs {
        factor: Some(factor),
        origin: "/usr/local/images/outputs".to_string(),
        dest: "/usr/local/images/outputs".to_string(),
        thread_count: None,
        image_size: 0,
    };

    let compressor = Compressor::new(args);
    compressor.compress().ok();
}
```

# License
Apache License, Version 2.0 ([LICENSE](LICENSE) or https://apache.org/licenses/LICENSE-2.0)
