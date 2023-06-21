# Rust Tools

## 配置镜像
  把 `config` 复制到 `$HOME/.cargo/config` 下。

## 设置 `workspace`
  - `cargo init -- --workspace`
  - 在 `Cargo.toml` 中添加
  ```toml
  [workspace]
   members = [
     "compress",
  ]
  ```

## 代码格式化
  - 安装
    ```shell
    cargo install rustfmt
    ```

  - 格式化单个文件
    ```shell
    rustfmt <filename>
    ```

  - 格式化目录
    ```shell
    rustfmt --edition 2021 --recursive <directory>
    ```
  
  - 格式化整个crate
    ```shell
    cargo fmt
    ```

## 打包
  ```shell
  cargo build
  cargo build --release --lib # 打成 lib
  cargo build --release --bin server # 打成可执行文件
  ```

## 运行项目
  ```shell
  cargo run
  ```

## 交叉编译
 前提: `brew install mingw-w64`。

 - windows
 ```shell
  rustup target add x86_64-pc-windows-gnu
  rustup toolchain install stable-x86_64-pc-windows-gnu
  cargo build --release --target x86_64-pc-windows-gnu
  cargo build --release --target x86_64-pc-windows-gnu --bin jjb-builder
 ```

 - linux
 ```shell
  rustup target add x86_64-unknown-linux-gnu
  cargo build --target x86_64-unknown-linux-gnu
  cargo build --release --target x86_64-unknown-linux-gnu --bin jjb-builder
 ```

## 打成 lib
  ```shell
  cargo build --release --lib --target x86_64-pc-windows-gnu
  ```

## 打包 wasm
  - 修改 `Cargo.toml`
    添加如下代码:
    ```toml
    [lib]
    crate-type = ["cdylib"]
    ```

    - 构建
    下面方法不生成 `init` 方法, 使用 `wasm-pack` 构建
    ```shell
    cargo install wasm-bindgen-cli --force
    cargo build --target wasm32-unknown-unknown --release
    wasm-bindgen target/wasm32-unknown-unknown/release/wasm.wasm --out-dir pkg 
    ```

    ```shell
    cargo install wasm-pack
    wasm-pack build --target nodejs
    ```

    - 测试
    * 测试项目
    ```shell
    wasm-pack test --chrome --headless
    ```
    * 测试指定文件
    ```shell
    cargo test --lib --test http_test
    ```
    
    - js 端使用(web)
    ```ts
    import __wbg_init, {greet} from '../../pkg/wasm.js'
    onMounted(async () => {
        await __wbg_init()
        greet('hello')
    })
    ```

## 生成文档
  ```shell
  cargo doc # --open 为打开
  ```

## 查看所有依赖
  ```shell
  cargo tree
  ```

# 输出到日志文件
  ```shell
  cargo run > output.txt
  ```

## 错误
  - Blocking waiting for file lock on package cache
  ```shell
    rm -rf ~/.cargo/.package-cache
  ```

## 集成 `Nodejs`
   使用 `napi` 生成 `.node` 文件。
   - 修改项目为 `lib`
     修改项目输出为 `lib.rs`。

   - 修改 `Cargo.toml` 文件
   ```toml
   [lib]
    crate-type = ["cdylib"]
    
    [dependencies]
    napi = "2"
    napi-derive = "2"
    
    [build-dependencies]
    napi-build = "1"
   ```
   - 在项目根目录添加 `build.rs` 文件
   ```rust
    // build.rs
    extern crate napi_build;
    
    fn main() {
        napi_build::setup();
    }
   ```

   - 添加 `package.json` 文件
   ```json
    {
      "package": "some-module",
      "devDependencies": {
        "@napi-rs/cli": "^1.0.0"
      },
      "napi": {
        "name": "module"
      },
      "scripts": {
        "build": "napi build --release",
        "build:debug": "napi build"
      }
    }
   ```
   - 生成 `.node` 文件
     运行 `npm run build` 或 `npm run build:debug`。