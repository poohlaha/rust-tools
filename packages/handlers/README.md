# Handlers
sftp file upload, File operation and some other public methods.

## Usage

To use `handles`, first add this to your `Cargo.toml`:

```toml
[dependencies]
handlers = {git = "https://github.com/poohlaha/rust-tools", branch = "main", package = "hanlders"}
```
Next, add this to your crate:

```rust
use handlers::file::{FileHandler, Error};

fn main() {
    // ...
}
```

# License
Apache License, Version 2.0 ([LICENSE](LICENSE) or https://apache.org/licenses/LICENSE-2.0)
