# `wasm` `http` 和 `https` 请求

## 修改 `Cargo.toml`

   ```toml
   [lib]
   crate-type = ["cdylib"]
   ```

## 打包成 `wasm`

   ```shell
   cargo install wasm-pack
   wasm-pack build --target web
   ```

## 测试

   ```shell
   cargo test
   # wasm-pack test --chrome --headless
   ```

## `js` 端使用
```ts
import __wbg_init, {send} from '../../pkg/wasm.js'
onMounted(async () => {
    await __wbg_init()
    send()
})
```