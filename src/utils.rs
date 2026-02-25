use crate::session_state::TypedSession;
use actix_web::error::ErrorInternalServerError;
use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;
use std::error::Error;

pub fn error_chain_fmt(error: &impl Error, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    writeln!(f, "{}\n", error)?;
    let mut current = error.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}

pub fn get_redirect_if_session_without_user_id(
    session: &TypedSession,
) -> Result<Option<HttpResponse>, actix_web::Error> {
    if session
        .get_user_id()
        .map_err(|error| ErrorInternalServerError(error))?
        .is_none()
    {
        return Ok(Some(
            HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .finish(),
        ));
    };

    Ok(None)
}
