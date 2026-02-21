use crate::helpers::spawn_server;

#[tokio::test]
async fn given_authentication_error_then_it_should_return_error_message_and_303() {
    let test_app = spawn_server().await;

    let request_body = serde_json::json!({
        "username": "mock_username",
        "password": "mock_password",
    });

    let response = test_app.post_login(&request_body).await;

    test_app.assert_is_redirect_to(&response, "/login");

    let html_page = test_app.get_login_html().await;

    assert!(html_page.contains(r#"<p><i>Authentication failed</i></p>"#));

    let html_page = test_app.get_login_html().await;

    assert!(!html_page.contains(r#"<p><i>Authentication failed</i></p>"#));
}
