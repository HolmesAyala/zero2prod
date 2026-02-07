use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    pub fn parse(value: String) -> Result<SubscriberName, String> {
        let is_empty_or_whitespace = value.trim().is_empty();
        let is_too_long = value.graphemes(true).count() > 256;

        let invalid_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}', '|'];
        let contains_invalid_characters = value
            .chars()
            .any(|character| invalid_characters.contains(&character));

        if is_empty_or_whitespace || is_too_long || contains_invalid_characters {
            Err(format!("The value is not a valid subscriber name. value = {}", value))
        } else {
            Ok(Self(value))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::SubscriberName;
    use claims::{assert_ok, assert_err};

    #[test]
    fn given_a_256_grapheme_value_then_it_should_return_ok() {
        let value = "ё".repeat(256);
        assert_ok!(SubscriberName::parse(value));
    }

    #[test]
    fn given_a_value_longer_than_256_then_it_should_return_err() {
        let value = "a".repeat(257);
        assert_err!(SubscriberName::parse(value));
    }

    #[test]
    fn given_a_value_with_whitespace_only_then_it_should_return_err() {
        let value = "   ".to_string();
        assert_err!(SubscriberName::parse(value));
    }

    #[test]
    fn given_a_value_with_invalid_characters_then_it_should_return_err() {
        let invalid_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}', '|'];

        for character in invalid_characters.iter() {
            let value = character.to_string();
            assert_err!(SubscriberName::parse(value));
        }
    }

    #[test]
    fn given_a_valid_value_then_it_should_return_ok() {
        let value = "Paul MaCarty".to_string();
        assert_ok!(SubscriberName::parse(value));
    }
}
