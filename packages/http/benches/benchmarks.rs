//! criterion 性能测试
use criterion::{criterion_group, criterion_main, Criterion};
use http::options::HttpResponse;
use http::options::Options;
use http::client::HttpClient;
use serde_json::json;
use tokio::runtime::Runtime;
use std::time::Duration;

// 异步性能测试函数
async fn send_benchmark() {
    let url = String::from("https://example.com");
    let data = json!({"test": "123456"});
    let options = Options {
        url,
        data: Some(data),
        method: Some("get".to_string()),
        headers: None,
    };
    let response: HttpResponse = HttpClient::send(options).await;
    assert_eq!(response.status_code, 200);
}

// 测试 send 函数
fn benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    c.bench_function("send_benchmark", |b| {
        b.iter(|| {
            rt.block_on(send_benchmark());
        });
    });

    let mut criterion_ref = c.benchmark_group("send_benchmark");
    criterion_ref.measurement_time(Duration::from_secs(10));
    criterion_ref.sample_size(200);
}

// 定义一个 criterion_group 宏来组织基准测试函数
criterion_group!(benches, benchmark);

// 使用 criterion_main 宏运行基准测试组
criterion_main!(benches);