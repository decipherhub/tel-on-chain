#[tokio::test]
async fn test_hello_api() {
    use reqwest::Client;
    // 서버가 백그라운드에서 실행 중이어야 함 (별도 프로세스)
    let res = Client::new()
        .get("http://localhost:8001/hello")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    assert!(res.contains("Hello, API!"));
}