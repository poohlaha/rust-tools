# Minimize
minimize css、html、js、json.

## Usage

To use `minimize`, first add this to your `Cargo.toml`:

```toml
[dependencies]
minimize = {git = "https://github.com/poohlaha/rust-tools", branch = "main", package = "minimize"}
```
Next, add this to your crate:

```rust
use minimize::minify::Minimize;

fn main() {
    // ...
}
```

# License
Apache License, Version 2.0 ([LICENSE](LICENSE) or https://apache.org/licenses/LICENSE-2.0)
