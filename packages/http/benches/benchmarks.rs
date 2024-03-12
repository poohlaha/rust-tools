use criterion::{criterion_group, criterion_main, Criterion};
use request_http::client::HttpClient;
use request_http::options::HttpResponse;
use request_http::options::Options;
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;

async fn send_benchmark() {
    let url = String::from("https://example.com");
    let data = json!({"test": "123456"});
    let options = Options {
        url,
        data: Some(data),
        method: Some("get".to_string()),
        headers: None,
        form: None,
        timeout: None,
    };
    let response: HttpResponse = HttpClient::send(options, false).await.unwrap();
    assert_eq!(response.status_code, 200);
}

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

criterion_group!(benches, benchmark);
criterion_main!(benches);
