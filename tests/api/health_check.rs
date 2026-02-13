use crate::helpers::spawn_server;

#[tokio::test]
async fn health_check_success() {
    let test_app = spawn_server().await;
    let service_url = format!("{}/health_check", test_app.address);
    let client = reqwest::Client::new();

    let response = client
        .get(service_url.clone())
        .send()
        .await
        .expect(&format!("Unable to perform the request to {}", service_url));

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}
