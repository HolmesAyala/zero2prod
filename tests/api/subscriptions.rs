use crate::helpers::spawn_server;

#[tokio::test]
async fn subscribe_success() {
    let test_app = spawn_server().await;
    let service_url = format!("{}/subscriptions", test_app.address);
    let http_client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = http_client
        .post(service_url.clone())
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect(&format!("Unable to perform the request to {}", service_url));

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
    let service_url = format!("{}/subscriptions", test_app.address);
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (requets_body, error_message) in test_cases {
        let response = client
            .post(service_url.clone())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(requets_body)
            .send()
            .await
            .expect(&format!("Unable to perform the request to {}", service_url));

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
    let app = spawn_server().await;
    let service_url = format!("{}/subscriptions", app.address);
    let http_client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, error_description) in test_cases {
        let response = http_client
            .post(service_url.clone())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect(&format!("Unable to perform the request to {}", service_url));

        assert_eq!(
            response.status().as_u16(),
            400,
            "The API did not return 400 Bad Request when the payload was {}",
            error_description
        )
    }
}
