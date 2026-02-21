use actix_web::http::header::ContentType;
use actix_web::HttpResponse;
use actix_web_flash_messages::{IncomingFlashMessages, Level};
use std::fmt::Write;

pub async fn login_form(flash_messages: IncomingFlashMessages) -> HttpResponse {
    let mut error_html = String::new();

    let error_messages = flash_messages
        .iter()
        .filter(|message| message.level() == Level::Error);

    for flash_message in error_messages {
        writeln!(error_html, "<p><i>{}</i></p>", flash_message.content()).unwrap()
    }

    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(format!(
            r#"
            <!DOCTYPE html>
            <html lang="en">
            <head>
                <meta http-equiv="content-type" content="text/html: charset=utf-8">
                <title>Login</title>
            </head>
            <body>
            <main>
                {error_html}
                <form action="/login" method="post">
                    <label>
                        Username
                        <input type="text" placeholder="Enter username" name="username"/>
                    </label>

                    <label>
                        Password
                        <input type="password" placeholder="Enter password" name="password"/>
                    </label>

                    <button type="submit">Login</button>
                </form>
            </main>
            </body>
            </html>
        "#
        ))
}
