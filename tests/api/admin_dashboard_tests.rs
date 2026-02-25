use crate::helpers::spawn_server;

#[tokio::test]
async fn given_an_unauthenticated_user_then_it_should_redirect_to_login() {
    let test_app = spawn_server().await;

    let response = test_app.get_admin_dashboard().await;

    test_app.assert_is_redirect_to(&response, "/login");
}
