use crate::domain::SubscriberEmail;
use reqwest::Client;
use secrecy::{ExposeSecret, SecretString};
use std::time::Duration;

pub struct EmailClient {
    http_client: Client,
    base_url: String,
    sender: SubscriberEmail,
    authorization_token: SecretString,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        authorization_token: SecretString,
        timeout: Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();

        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequestBody {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };

        self.http_client
            .post(url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequestBody<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use claims::{assert_err, assert_ok};
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use secrecy::SecretString;
    use std::time::Duration;
    use wiremock::matchers::{any, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct SendEmailRequestBodyMatcher {
        expected_body: serde_json::Value,
    }

    impl SendEmailRequestBodyMatcher {
        pub fn create(expected_body: serde_json::Value) -> SendEmailRequestBodyMatcher {
            SendEmailRequestBodyMatcher { expected_body }
        }
    }

    impl wiremock::Match for SendEmailRequestBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let request_body_result: Result<serde_json::value::Value, _> =
                serde_json::from_slice(&request.body);

            if let Ok(request_body) = request_body_result {
                request_body.get("From") == self.expected_body.get("From")
                    && request_body.get("To") == self.expected_body.get("To")
                    && request_body.get("Subject") == self.expected_body.get("Subject")
                    && request_body.get("HtmlBody") == self.expected_body.get("HtmlBody")
                    && request_body.get("TextBody") == self.expected_body.get("TextBody")
            } else {
                false
            }
        }
    }

    #[tokio::test]
    async fn then_it_should_send_email() {
        let mock_server = MockServer::start().await;
        let sender_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let authorization_token_mock: String = Faker.fake::<String>();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender_email.clone(),
            SecretString::from(authorization_token_mock.clone()),
            Duration::from_millis(200),
        );
        let subscriber_email = email();
        let subject: String = subject();
        let content: String = content();

        let request_body_expected = serde_json::json!({
            "From": sender_email.as_ref(),
            "To": subscriber_email.as_ref(),
            "Subject": subject,
            "HtmlBody": content,
            "TextBody": content,
        });

        Mock::given(header("X-Postmark-Server-Token", authorization_token_mock))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailRequestBodyMatcher::create(request_body_expected))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_ok!(result);
    }

    #[tokio::test]
    async fn given_a_500_response_then_it_should_return_error() {
        let mock_server = MockServer::start().await;
        let sender_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let authorization_token_mock: String = Faker.fake::<String>();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender_email.clone(),
            SecretString::from(authorization_token_mock.clone()),
            Duration::from_millis(200),
        );

        let subscriber_email = email();
        let subject: String = subject();
        let content: String = content();

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(result);
    }

    #[tokio::test]
    async fn given_a_response_timeout_then_it_should_return_error() {
        let mock_server = MockServer::start().await;
        let sender_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let authorization_token_mock: String = Faker.fake::<String>();
        let email_client = EmailClient::new(
            mock_server.uri(),
            sender_email.clone(),
            SecretString::from(authorization_token_mock.clone()),
            Duration::from_millis(200),
        );

        let subscriber_email = email();
        let subject: String = subject();
        let content: String = content();

        let response = ResponseTemplate::new(200).set_delay(Duration::from_secs(11));

        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let result = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        assert_err!(result);
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }
}
