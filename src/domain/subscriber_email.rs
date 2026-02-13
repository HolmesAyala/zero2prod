use validator::ValidateEmail;

#[derive(Debug, Clone)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(value: String) -> Result<SubscriberEmail, String> {
        if ValidateEmail::validate_email(&value) {
            Ok(Self(value))
        } else {
            Err(format!("The value provided is not a valid email address. value = {}", value))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use claims::{assert_err, assert_ok};
    use fake::Fake;
    use fake::faker::internet::en::SafeEmail;
    use quickcheck::Gen;

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidEmailFixture {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let email = SafeEmail().fake_with_rng(g);
            Self(email)
        }
    }

    #[test]
    fn given_an_empty_value_then_it_should_return_err() {
        let value = "".to_string();
        assert_err!(SubscriberEmail::parse(value));
    }

    #[test]
    fn given_the_at_character_is_missing_then_it_should_return_err() {
        let value = "somevalue".to_string();
        assert_err!(SubscriberEmail::parse(value));
    }

    #[test]
    fn given_a_missing_subject_then_it_should_return_err() {
        let value = "@mail.com".to_string();
        assert_err!(SubscriberEmail::parse(value));
    }

    #[quickcheck_macros::quickcheck]
    fn given_a_valid_value_then_it_should_return_ok(valid_email: ValidEmailFixture) {
        assert_ok!(SubscriberEmail::parse(valid_email.0));
    }
}
