//! 测试 http 请求
use tokio::runtime::Runtime;
use http::options::HttpResponse;
use http::options::Options;
use http::client::HttpClient;

#[test]
fn test_http_get() {
    let run_time = Runtime::new().unwrap();
    run_time.block_on(async {
        let url = String::from("https://example.com");
        let options = Options {
            url,
            data: None,
            method: Some("get".to_string()),
            headers: None,
        };
        let response: HttpResponse = HttpClient::send(options).await;
        assert_eq!(response.status_code, 200);
    });
}

#[test]
fn test_http_post() {
    let run_time = Runtime::new().unwrap();
    run_time.block_on(async {
        let url = String::from("https://example.com");
        let data = serde_json::json!({
          "data": {
            "openId":"8888888888"
          },
          "requestTime": "202306171000",
          "version": "1.0"
        });

        let options = Options {
            url,
            data: Some(data),
            method: None,
            headers: None,
        };
        let response: HttpResponse = HttpClient::send(options).await;
        assert_eq!(response.status_code, 200);
    });
}