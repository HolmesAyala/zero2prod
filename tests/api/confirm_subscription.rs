use crate::helpers::spawn_server;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn given_a_missing_token_then_it_should_return_400() {
    let test_app = spawn_server().await;
    let url = format!("{}/subscriptions/confirm", &test_app.address);

    let response = reqwest::get(url).await.unwrap();

    assert_eq!(400, response.status());
}

#[tokio::test]
async fn given_a_valid_confirmation_link_then_it_should_return_200() {
    let test_app = spawn_server().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.to_owned()).await;

    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);

    let response = reqwest::get(confirmation_links.html).await.unwrap();

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn then_it_should_confirm_subscription() {
    let test_app = spawn_server().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.to_owned()).await;
    let email_request = &test_app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = test_app.get_confirmation_links(email_request);

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let record = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&test_app.db_connection_pool)
        .await
        .expect("Failed to fetch record");

    assert_eq!("le guin", record.name);
    assert_eq!("ursula_le_guin@gmail.com", record.email);
    assert_eq!("confirmed", record.status);
}
