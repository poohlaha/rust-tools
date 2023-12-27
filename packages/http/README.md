# request-http

send a `http` or `https` request, include `form-data`, `form submit`.

## Usage

To use `http`, first add this to your `Cargo.toml`:

```toml
[dependencies]
request-http = "0.1"
```
Next, add this to your crate:

```rust
use request_http::{client_send, client_send_form_data, download};

fn main() {
    // ...
}
```

## Examples
Create an HTTP `get` request:

```rust
use request_http::client_send;

#[tokio::main]
async fn main() {
    let url = String::from("https://www.rust-lang.org/");
    let options = Options {
        url,
        data: None,
        form: None,
        method: Some("get".to_string()),
        headers: None,
        timeout: None,
    };
    let response: HttpResponse = client_send(options, false).await?;
}
```

Create an HTTP `post` request:

```rust
use request_http::client_send;

#[tokio::main]
async fn main() {
    let url = String::from("https://www.rust-lang.org/");
    let data = serde_json::json!({
          "data": {
            "test":"123456"
          },
          "requestTime": "202306171000",
          "version": "1.0"
        });

    let options = Options {
        url,
        data: Some(data),
        form: None,
        method: None,
        headers: None,
        timeout: None,
    };
    let response: HttpResponse = client_send(options, false).await?;
}
```

Create an HTTP `form-data` request:

```rust
use request_http::{client_send_form_data, HttpFormData};

#[tokio::main]
async fn main() {
    let url = String::from("https://example.com/api/upload");
    let form = HttpFormData::new()
        .text("userId", "10074")
        .text("version", "1.0")
        .file("files", "/usr/local/text.zip")
        .unwrap();

    let options = Options {
        url,
        data: None,
        form: Some(form),
        method: None,
        headers: None,
        timeout: None,
    };
    let response: HttpResponse = client_send_form_data(options)?;
}
```

Download `file` with progress bar:

```rust
use request_http::download;

#[tokio::main]
async fn main() {
    download(
        DownloadOptions {
            url: "https://example.com/api/download",
            file_name: None,
            timeout: None,
            output_dir: Some(args.workspace.clone()),
            overwrite: Some(true),
        },
        None, // if u use process bar, please create `MultiProgress`
    ).await?;
}
```

# License
Apache License, Version 2.0 ([LICENSE](LICENSE) or https://apache.org/licenses/LICENSE-2.0)
