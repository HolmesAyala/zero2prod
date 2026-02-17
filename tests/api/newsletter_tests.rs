use crate::helpers::{ConfirmationLinks, TestApp, spawn_server};
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn then_it_should_not_send_newsletter_to_unconfirmed_subscribers() {
    let test_app = spawn_server().await;
    create_subscription_request(&test_app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter plain content",
            "html": "<p>Newsletter HTML content</p>",
        }
    });

    let response = test_app.post_newsletter(newsletter_request_body).await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn then_it_should_send_newsletter_to_confirmed_subscribers() {
    let test_app = spawn_server().await;

    create_and_confirm_subscription(&test_app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&test_app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "content": {
            "text": "Newsletter plain content",
            "html": "<p>Newsletter HTML content</p>",
        }
    });

    let response = test_app.post_newsletter(newsletter_request_body).await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn given_an_invalid_request_body_then_it_should_returns_400() {
    let test_app = spawn_server().await;

    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Content",
                    "html": "<p>Content</p>",
                }
            }),
            "Missing title",
        ),
        (
            serde_json::json!({
                "title": "Title"
            }),
            "Missing content",
        ),
    ];

    for test_case in test_cases {
        let response = test_app.post_newsletter(test_case.0).await;

        assert_eq!(
            response.status(),
            reqwest::StatusCode::BAD_REQUEST,
            "The API did not fail with 400 Bad Request when the request body was {}",
            test_case.1
        );
    }
}

async fn create_subscription_request(test_app: &TestApp) -> ConfirmationLinks {
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscribers")
        .expect(1)
        .mount_as_scoped(&test_app.email_server)
        .await;

    test_app
        .post_subscriptions(body.to_owned())
        .await
        .error_for_status()
        .unwrap();

    let confirm_email_request = test_app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    test_app.get_confirmation_links(&confirm_email_request)
}

async fn create_and_confirm_subscription(test_app: &TestApp) {
    let confirmation_links = create_subscription_request(test_app).await;

    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}
