use minicbor::{CborLen, Decode, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Newtype for an email address
/// It is backed by a String and implements a PartialEq instance
/// based on a lowercase comparison.
///
/// Note that the SMTP RFC (https://www.rfc-editor.org/rfc/rfc5321.txt, section 2.4) specifies that
/// we should not make a case insensitive comparison on the local part of the email address
///
/// However we currently receive lowercase email addresses from the Controller in `ProjectUserRole`,
/// and we need to make a case insensitive comparison when comparing with an email address in
/// `UserInfo`.
#[derive(Debug, Clone, Eq, Ord, PartialOrd, Deserialize, Encode, Decode, CborLen, Serialize)]
#[cbor(transparent)]
#[serde(transparent)]
pub struct EmailAddress(#[n(0)] String);

impl EmailAddress {
    /// Create a new EmailAddress without parsing it
    /// To be used with care!
    pub fn new_unsafe(s: &str) -> EmailAddress {
        EmailAddress(s.to_string())
    }

    /// Parse an email address using the email_address crate
    pub fn parse(s: &str) -> Result<EmailAddress> {
        // validate the email address using the same regex as the one used by the Controller
        // https://html.spec.whatwg.org/multipage/input.html#valid-e-mail-address
        let regex = Regex::new(r"^[a-zA-Z0-9.!#$%&'*+\\/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$").map_err(Self::incorrect_regex)?;
        if regex.is_match(s) {
            Ok(EmailAddress(s.to_string()))
        } else {
            Err(Error::new(
                Origin::Api,
                Kind::Invalid,
                format!("{s} is not a valid email address"),
            ))
        }
    }

    fn incorrect_regex(e: regex::Error) -> Error {
        Error::new(
            Origin::Api,
            Kind::Invalid,
            format!("incorrect regular expression {e:?}"),
        )
    }
}

/// Lowercase comparison for email addresses
impl PartialEq for EmailAddress {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

impl Display for EmailAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<String> for EmailAddress {
    type Error = Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        EmailAddress::parse(value.as_str())
    }
}

impl TryFrom<&str> for EmailAddress {
    type Error = Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        EmailAddress::parse(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::{quickcheck, Arbitrary, Gen, TestResult};

    quickcheck! {
        /// Valid addresses should be parsed ok
        fn parse_valid_address(email_address: ValidEmailAddress) -> TestResult {
            match EmailAddress::parse(&email_address.0.0) {
                Ok(_) => TestResult::passed(),
                Err(e) => TestResult::error(format!("{e:?}")),
            }
        }

        /// Invalid addresses should fail when parsed
        fn parse_invalid_address(email_address: InvalidEmailAddress) -> TestResult {
            match EmailAddress::parse(&email_address.0.0) {
                Ok(_) => TestResult::error(format!("the email address {} should not be valid", email_address.0)),
                Err(_) => TestResult::passed(),
            }
        }

        /// Check that lowercase comparison is done to determine if 2 email addresses are equal
        fn email_equality(email_address1: EqualEmailAddress, email_address2: EqualEmailAddress) -> bool {
            email_address1.0 == email_address2.0
        }
    }

    /// HELPERS

    /// This newtype generates equal email addresses
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct EqualEmailAddress(EmailAddress);

    impl Arbitrary for EqualEmailAddress {
        fn arbitrary(gen: &mut Gen) -> Self {
            EqualEmailAddress(EmailAddress(
                gen.choose(&[
                    "test@ockam.io",
                    "test@ockam.io",
                    "tEst@ockam.io",
                    "test@Ockam.io",
                    "TEST@OCKAM.IO",
                ])
                .unwrap()
                .to_string(),
            ))
        }
    }

    /// This newtype generates valid email addresses according to the HTML5 regex
    /// The examples are taken from: https://en.wikipedia.org/wiki/Email_address#Valid_email_addresses
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct ValidEmailAddress(EmailAddress);

    impl Arbitrary for ValidEmailAddress {
        fn arbitrary(gen: &mut Gen) -> Self {
            ValidEmailAddress(EmailAddress(
                gen.choose(&[
                    "simple@example.com",
                    "very.common@example.com",
                    "x@example.com",
                    "long.email-address-with-hyphens@and.subdomains.example.com",
                    "user.name+tag+sorting@example.com",
                    "name/surname@example.com",
                    "admin@example",
                    "example@s.example",
                    "mailhost!username@example.org",
                    "user%example.com@example.org",
                    "user-@example.org",
                    // these examples come from the list of valid email addresses on https://en.wikipedia.org/wiki/Email_address#Valid_email_addresses
                    // but actually fail to parse with the HTML5 regex
                    // "\" \"@example.org",
                    // "\"john..doe\"@example.org",
                    // "\"very.(),:;<>[]\\\".VERY.\\\"very@\\ \\\"very\\\".unusual\"@strange.example.com",
                    // "postmaster@[123.123.123.123]",
                    // "postmaster@[IPv6:2001:0db8:85a3:0000:0000:8a2e:0370:7334]",
                    // "_test@[IPv6:2001:0db8:85a3:0000:0000:8a2e:0370:7334]",
                ])
                .unwrap()
                .to_string(),
            ))
        }
    }

    /// This newtype generates invalid email addresses
    /// The examples are taken from: https://en.wikipedia.org/wiki/Email_address#Invalid_email_addresses
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct InvalidEmailAddress(EmailAddress);

    impl Arbitrary for InvalidEmailAddress {
        fn arbitrary(gen: &mut Gen) -> Self {
            InvalidEmailAddress(EmailAddress(
                gen.choose(&[
                    "abc.example.com",
                    "a@b@c@example.com",
                    "a\"b(c)d,e:f;g<h>i[j\\k]l@example.com",
                    "just\"not\"right@example.com",
                    "this is\"not\\allowed@example.com",
                    "this\\ still\\\"not\\allowed@example.com",
                    "i.like.underscores@but_they_are_not_allowed_in_this_part",
                    // this example comes from the list of invalid email addresses on https://en.wikipedia.org/wiki/Email_address#Invalid_email_addresses
                    // but actually parses ok with the HTML5 regex
                    // "1234567890123456789012345678901234567890123456789012345678901234+x@example.com",
                ])
                .unwrap()
                .to_string(),
            ))
        }
    }

    /// Arbitrary instances for EmailAddress
    /// All the addresses must be parseable
    impl Arbitrary for EmailAddress {
        fn arbitrary(gen: &mut Gen) -> Self {
            EmailAddress(
                gen.choose(&["test@ockam.io", "user@yahoo.com", "ceo@google.com"])
                    .unwrap()
                    .to_string(),
            )
        }
    }
}
