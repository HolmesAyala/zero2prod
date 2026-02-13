use crate::helpers::spawn_server;

#[tokio::test]
async fn given_a_valid_request_then_it_subscribes() {
    let test_app = spawn_server().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = test_app.post_subscriptions(body.to_owned()).await;

    assert_eq!(response.status().as_u16(), 200);

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&test_app.db_connection_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
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
