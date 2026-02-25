use crate::session_state::TypedSession;
use crate::utils::get_redirect_if_session_without_user_id;
use actix_web::web::Form;
use actix_web::HttpResponse;
use secrecy::SecretString;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct FormData {
    current_password: SecretString,
    new_password: SecretString,
    new_password_check: SecretString,
}

pub async fn change_password(
    form: Form<FormData>,
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    if let Some(redirect_response) = get_redirect_if_session_without_user_id(&session)? {
        return Ok(redirect_response);
    }

    todo!()
}
