//! 测试 http 请求
use request_http::options::HttpResponse;
use request_http::options::Options;
use request_http::{client_send, client_send_form_data, HttpFormData};
use tokio::runtime::Runtime;

#[test]
fn test_http_get() {
    let run_time = Runtime::new().unwrap();
    run_time.block_on(async {
        let url = String::from("https://example.com");
        let options = Options {
            url,
            data: None,
            form: None,
            method: Some("get".to_string()),
            headers: None,
            timeout: None,
        };
        let response: HttpResponse = client_send(options, false).await.unwrap();
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
            form: None,
            method: None,
            headers: None,
            timeout: None,
        };
        let response: HttpResponse = client_send(options, false).await.unwrap();
        assert_eq!(response.status_code, 200);
    });
}

#[test]
fn test_http_form_data() {
    let url = String::from("http://example.com/api/upload");
    let form = HttpFormData::new().text("userId", "10074").text("version", "1.0").file("files", "/usr/local/text.zip").unwrap();

    let options = Options {
        url,
        data: None,
        form: Some(form),
        method: None,
        headers: None,
        timeout: None,
    };
    let response: HttpResponse = client_send_form_data(options).unwrap();
    assert_eq!(response.status_code, 200);
}
