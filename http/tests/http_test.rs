//! 测试 http 请求
use tokio::runtime::Runtime;
use http::options::HttpResponse;
use http::options::Options;
use http::client::HttpClient;

#[test]
fn test_http_get() {
    let run_time = Runtime::new().unwrap();
    run_time.block_on(async {
        let url = String::from("https://openapitest.xcsc.com:9090/open/customer/api/custAuth/validateImg");
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
        let url = String::from("https://openapitest.xcsc.com:9090/open/customer/api/custAccountBind/queryAccountBind");
        let data = serde_json::json!({
          "appId": "1",
          "appVersion": "1",
          "channel": "app",
          "channelId": "NA",
          "clientId": "10000069",
          "cpuNo": "1",
          "data": {
            "openId":"8888888888"
          },
          "deviceId": "NA",
          "diskId": "NA",
          "fundAccount": "10000069",
          "fundToken": "__xCsc_46f39cf870164e8f8b1e1558a312aeb7",
          "localIp": "127.0.0.1",
          "loginAccount": "",
          "loginToken": "",
          "macAddr": "NA",
          "opStation": "NA",
          "requestTime": "202108171000",
          "version": "1.0",
          "requestId" : "5d9d4542-8a4f-484d-ac27-043d0cdca25f"
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