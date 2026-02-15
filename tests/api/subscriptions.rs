use crate::helpers::spawn_server;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn given_a_valid_request_body_then_it_return_200() {
    let test_app = spawn_server().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscriptions(body.to_owned()).await;

    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn then_it_should_save_the_new_subscriber() {
    let test_app = spawn_server().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    test_app.post_subscriptions(body.to_owned()).await;

    let record_saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&test_app.db_connection_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(record_saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(record_saved.name, "le guin");
    assert_eq!(record_saved.status, "pending_confirmation");
}

#[tokio::test]
async fn given_a_new_subscription_then_it_should_send_a_confirmation_email() {
    let test_app = spawn_server().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscriptions(body.to_owned()).await;

    assert_eq!(200, response.status());
}

#[tokio::test]
async fn given_a_new_subscription_then_it_should_send_a_confirmation_email_with_a_link() {
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

    assert_eq!(confirmation_links.plain_text, confirmation_links.html);
}

#[tokio::test]
async fn given_missing_fields_then_subscribe_returns_400() {
    let test_app = spawn_server().await;

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (requets_body, error_message) in test_cases {
        let response = test_app.post_subscriptions(requets_body.to_owned()).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        )
    }
}

#[tokio::test]
async fn given_fields_invalid_then_subscribe_returns_400() {
    let test_app = spawn_server().await;

    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, error_description) in test_cases {
        let response = test_app.post_subscriptions(body.to_owned()).await;

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not return 400 Bad Request when the payload was {}",
            error_description
        )
    }
}
