use crate::i18n;

// Constants
pub const MAX_VALUE_LENGTH: usize = 256;
const MAX_HOSTNAME_LENGTH: usize = 253;
const MAX_LABEL_LENGTH: usize = 63;
const MAX_INTEGER_DIGITS: usize = 10;

// Kind
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Hostname,
    Ipv4,
    Host,
    Integer { min: u32, max: u32 },
    Token { max_length: usize },
    Choice { values: Vec<String> },
}

impl Kind {
    pub fn check(&self, value: &str) -> Result<(), &'static str> {
        if !is_safe(value) {
            return Err(i18n::REJECTION_VALUE_UNSAFE);
        }
        let valid = match self {
            Self::Hostname => is_hostname(value),
            Self::Ipv4 => is_ipv4(value),
            Self::Host => is_hostname(value) || is_ipv4(value),
            Self::Integer { min, max } => {
                parse_integer(value).is_some_and(|number| (*min..=*max).contains(&number))
            }
            Self::Token { max_length } => value.len() <= *max_length && is_token(value),
            Self::Choice { values } => values.iter().any(|allowed| allowed == value),
        };
        if valid {
            Ok(())
        } else {
            Err(self.expectation())
        }
    }

    fn expectation(&self) -> &'static str {
        match self {
            Self::Hostname => i18n::REJECTION_VALUE_HOSTNAME,
            Self::Ipv4 => i18n::REJECTION_VALUE_IPV4,
            Self::Host => i18n::REJECTION_VALUE_HOST,
            Self::Integer { .. } => i18n::REJECTION_VALUE_INTEGER,
            Self::Token { .. } => i18n::REJECTION_VALUE_TOKEN,
            Self::Choice { .. } => i18n::REJECTION_VALUE_CHOICE,
        }
    }
}

// Rules
pub fn is_safe(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_VALUE_LENGTH
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn is_hostname(value: &str) -> bool {
    value.len() <= MAX_HOSTNAME_LENGTH && value.split('.').all(is_label)
}

fn is_label(label: &str) -> bool {
    (1..=MAX_LABEL_LENGTH).contains(&label.len())
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn is_ipv4(value: &str) -> bool {
    let mut count = 0;
    for part in value.split('.') {
        count += 1;
        if count > 4 || part.len() > 3 || parse_integer(part).is_none_or(|number| number > 255) {
            return false;
        }
    }
    count == 4
}

fn is_token(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub fn parse_integer(value: &str) -> Option<u32> {
    let plain = !value.is_empty()
        && value.len() <= MAX_INTEGER_DIGITS
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'));
    if plain { value.parse().ok() } else { None }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn accepts(kind: &Kind, values: &[&str]) {
        for value in values {
            assert_eq!(kind.check(value), Ok(()), "{value}");
        }
    }

    fn rejects(kind: &Kind, values: &[&str]) {
        for value in values {
            assert!(kind.check(value).is_err(), "{value}");
        }
    }

    #[test]
    fn rejects_unsafe_characters_for_every_kind() {
        let kinds = [
            Kind::Hostname,
            Kind::Ipv4,
            Kind::Host,
            Kind::Integer { min: 0, max: 9 },
            Kind::Token { max_length: 64 },
            Kind::Choice {
                values: vec!["a b".to_owned(), "-x".to_owned(), "ä".to_owned()],
            },
        ];
        let values = [
            "",
            "a b",
            "a\tb",
            "a\"b",
            "a'b",
            "a`b",
            "a\\b",
            "a/b",
            "a%b",
            "a^b",
            "a&b",
            "a|b",
            "a<b",
            "a>b",
            "a;b",
            "a,b",
            "a=b",
            "a(b",
            "a)b",
            "a!b",
            "a*b",
            "a?b",
            "a:b",
            "a@b",
            "a#b",
            "a$b",
            "a{b",
            "a~b",
            "a+b",
            "a\nb",
            "a\0b",
            "-x",
            "ä",
            "pc\u{202e}",
        ];
        for kind in &kinds {
            rejects(kind, &values);
        }
    }

    #[test]
    fn checks_host_names() {
        let kind = Kind::Hostname;
        accepts(
            &kind,
            &["pc0042", "PC0042", "a", "pc-1.example.com", "1a.2b", "123"],
        );
        rejects(
            &kind,
            &["pc_1", ".pc", "pc.", "pc..de", "-pc", "pc-", "a.-b", "a-.b"],
        );
        accepts(&kind, &[&"a".repeat(63)]);
        rejects(&kind, &[&"a".repeat(64)]);
        let long = [
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(61),
        ]
        .join(".");
        assert_eq!(long.len(), 253);
        accepts(&kind, &[&long]);
    }

    #[test]
    fn limits_the_length_of_every_value() {
        let long = [
            "a".repeat(63),
            "b".repeat(63),
            "c".repeat(63),
            "d".repeat(63),
        ]
        .join(".");
        rejects(&Kind::Hostname, &[&long]);
        rejects(&Kind::Token { max_length: 1000 }, &[&"a".repeat(257)]);
    }

    #[test]
    fn checks_ipv4_addresses() {
        let kind = Kind::Ipv4;
        accepts(&kind, &["192.168.50.101", "0.0.0.0", "255.255.255.255"]);
        rejects(
            &kind,
            &[
                "256.1.1.1",
                "1.1.1",
                "1.1.1.1.1",
                "01.1.1.1",
                "1.1.1.",
                "1..1.1",
                "a.b.c.d",
                "0x7f.0.0.1",
                "1.1.1.1000",
            ],
        );
    }

    #[test]
    fn host_accepts_names_and_addresses() {
        accepts(&Kind::Host, &["pc0042", "10.0.0.1", "pc.example.com"]);
        rejects(&Kind::Host, &["pc_1", "-pc"]);
    }

    #[test]
    fn checks_integers() {
        let kind = Kind::Integer { min: 1, max: 65535 };
        accepts(&kind, &["1", "3389", "65535"]);
        rejects(
            &kind,
            &["0", "65536", "03389", "1.5", "1e3", "abc", "99999999999"],
        );
        accepts(&Kind::Integer { min: 0, max: 0 }, &["0"]);
        rejects(&Kind::Integer { min: 0, max: 9 }, &["00"]);
    }

    #[test]
    fn checks_tokens() {
        let kind = Kind::Token { max_length: 5 };
        accepts(&kind, &["a", "A-b_1", "12345"]);
        rejects(&kind, &["123456", "a.b", "-ab"]);
    }

    #[test]
    fn checks_choices() {
        let kind = Kind::Choice {
            values: vec!["view".to_owned(), "control".to_owned()],
        };
        accepts(&kind, &["view", "control"]);
        rejects(&kind, &["View", "admin", "vie", "viewx"]);
    }
}
