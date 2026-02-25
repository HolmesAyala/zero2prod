use crate::helpers::spawn_server;
use uuid::Uuid;

#[tokio::test]
async fn given_an_unauthenticated_user_requesting_the_change_password_form_then_it_should_redirect_to_login()
 {
    let test_app = spawn_server().await;

    let response = test_app.get_change_password().await;

    test_app.assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn given_an_unauthenticated_user_requesting_to_change_password_then_it_should_redirect_to_login()
 {
    let test_app = spawn_server().await;

    let new_password = Uuid::new_v4().to_string();

    let request_body = serde_json::json!({
        "current_password": Uuid::new_v4().to_string(),
        "new_password": &new_password,
        "new_password_check": &new_password,
    });

    let response = test_app.post_change_password(&request_body).await;

    test_app.assert_is_redirect_to(&response, "/login");
}
