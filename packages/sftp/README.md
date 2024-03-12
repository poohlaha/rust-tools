# Sftp
sftp file upload.

## Usage

To use `sftp`, first add this to your `Cargo.toml`:

```toml
[dependencies]
sftp = {git = "https://github.com/poohlaha/rust-tools", branch = "main", package = "sftp"}
```
Next, add this to your crate:

```rust
use sftt::file::{FileHandler, Error};

fn main() {
    // ...
}
```

# License
Apache License, Version 2.0 ([LICENSE](LICENSE) or https://apache.org/licenses/LICENSE-2.0)
