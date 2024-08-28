# docker
Package the docker image and modify the kubectl pod

## Usage

To use `docker`, first add this to your `Cargo.toml`:

```toml
[dependencies]
handlers = {git = "https://github.com/poohlaha/rust-tools", branch = "main", package = "docker"}
```
Next, add this to your crate:

```rust
use docker::docker::DockerHandler;
use docker::DockerConfig;

fn main() {
    // ...
}
```

# License
Apache License, Version 2.0 ([LICENSE](LICENSE) or https://apache.org/licenses/LICENSE-2.0)
