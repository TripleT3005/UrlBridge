use std::fmt;

// Constants
pub const MAX_LENGTH: usize = 2048;
const AUTHORITY_PREFIX: &str = "//";

// Request
#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub scheme: String,
    pub values: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UrlError {
    TooLong,
    InvalidCharacter,
    MissingScheme,
    MissingSlashes,
    Query,
    Fragment,
    UserInfo,
    EmptyValue,
    InvalidEncoding,
}

impl fmt::Display for UrlError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::TooLong => "The URL is longer than 2048 characters.",
            Self::InvalidCharacter => "The URL contains spaces, control or non-ASCII characters.",
            Self::MissingScheme => "The URL has no valid scheme.",
            Self::MissingSlashes => "The URL must have the form scheme://value.",
            Self::Query => "The URL contains a query (?).",
            Self::Fragment => "The URL contains a fragment (#).",
            Self::UserInfo => "The URL contains user information (@).",
            Self::EmptyValue => "The URL contains an empty value.",
            Self::InvalidEncoding => "The URL contains invalid percent-encoding.",
        })
    }
}

// Parsing
pub fn parse(url: &str) -> Result<Request, UrlError> {
    if url.len() > MAX_LENGTH {
        return Err(UrlError::TooLong);
    }
    if !url.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(UrlError::InvalidCharacter);
    }
    let (scheme, rest) = url.split_once(':').ok_or(UrlError::MissingScheme)?;
    if !is_scheme(scheme) {
        return Err(UrlError::MissingScheme);
    }
    let rest = rest
        .strip_prefix(AUTHORITY_PREFIX)
        .ok_or(UrlError::MissingSlashes)?;
    if rest.contains('?') {
        return Err(UrlError::Query);
    }
    if rest.contains('#') {
        return Err(UrlError::Fragment);
    }
    if rest.contains('@') {
        return Err(UrlError::UserInfo);
    }
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let values = if rest.is_empty() {
        Vec::new()
    } else {
        rest.split('/').map(decode).collect::<Result<_, _>>()?
    };
    Ok(Request {
        scheme: scheme.to_ascii_lowercase(),
        values,
    })
}

fn is_scheme(scheme: &str) -> bool {
    scheme
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphabetic())
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn decode(segment: &str) -> Result<String, UrlError> {
    if segment.is_empty() {
        return Err(UrlError::EmptyValue);
    }
    let bytes = segment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex(bytes.get(index + 1).copied())?;
            let low = hex(bytes.get(index + 2).copied())?;
            decoded.push(high * 16 + low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).map_err(|_| UrlError::InvalidEncoding)
}

fn hex(byte: Option<u8>) -> Result<u8, UrlError> {
    byte.and_then(|byte| char::from(byte).to_digit(16))
        .and_then(|digit| u8::try_from(digit).ok())
        .ok_or(UrlError::InvalidEncoding)
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn values(url: &str) -> Vec<String> {
        parse(url).unwrap().values
    }

    #[test]
    fn parses_scheme_and_values() {
        assert_eq!(
            parse("bridge-rdp://pc0042"),
            Ok(Request {
                scheme: "bridge-rdp".to_owned(),
                values: vec!["pc0042".to_owned()],
            })
        );
        assert_eq!(values("a://x/y/z"), ["x", "y", "z"]);
    }

    #[test]
    fn lowercases_the_scheme_only() {
        let request = parse("Bridge-RDP://PC0042").unwrap();
        assert_eq!(request.scheme, "bridge-rdp");
        assert_eq!(request.values, ["PC0042"]);
    }

    #[test]
    fn ignores_one_trailing_slash() {
        assert_eq!(values("a://x/"), ["x"]);
        assert_eq!(values("a://"), Vec::<String>::new());
        assert_eq!(values("a:///"), Vec::<String>::new());
        assert_eq!(parse("a://x//"), Err(UrlError::EmptyValue));
        assert_eq!(parse("a:////"), Err(UrlError::EmptyValue));
    }

    #[test]
    fn rejects_empty_values() {
        assert_eq!(parse("a://x//y"), Err(UrlError::EmptyValue));
        assert_eq!(parse("a:///x"), Err(UrlError::EmptyValue));
    }

    #[test]
    fn decodes_percent_encoding_once() {
        assert_eq!(values("a://pc%2D1"), ["pc-1"]);
        assert_eq!(values("a://a%2Fb"), ["a/b"]);
        assert_eq!(values("a://a%2520b"), ["a%20b"]);
        assert_eq!(values("a://a%20b"), ["a b"]);
        assert_eq!(values("a://%C3%A4"), ["ä"]);
    }

    #[test]
    fn rejects_invalid_encoding() {
        for url in ["a://%", "a://%2", "a://%zz", "a://%FF", "a://%C3"] {
            assert_eq!(parse(url), Err(UrlError::InvalidEncoding), "{url}");
        }
    }

    #[test]
    fn rejects_query_fragment_and_user_info() {
        assert_eq!(parse("a://x?y=1"), Err(UrlError::Query));
        assert_eq!(parse("a://x#y"), Err(UrlError::Fragment));
        assert_eq!(parse("a://user@x"), Err(UrlError::UserInfo));
    }

    #[test]
    fn rejects_malformed_urls() {
        assert_eq!(parse("pc0042"), Err(UrlError::MissingScheme));
        assert_eq!(parse("://x"), Err(UrlError::MissingScheme));
        assert_eq!(parse("1a://x"), Err(UrlError::MissingScheme));
        assert_eq!(parse("a_b://x"), Err(UrlError::MissingScheme));
        assert_eq!(parse("a:x"), Err(UrlError::MissingSlashes));
        assert_eq!(parse("a:/x"), Err(UrlError::MissingSlashes));
    }

    #[test]
    fn rejects_spaces_control_and_non_ascii_characters() {
        for url in ["a://x y", "a://x\ty", "a://x\n", "a://ä", "a://x\0"] {
            assert_eq!(parse(url), Err(UrlError::InvalidCharacter), "{url}");
        }
    }

    #[test]
    fn limits_the_length() {
        let fits = format!("a://{}", "x".repeat(MAX_LENGTH - 4));
        assert!(parse(&fits).is_ok());
        let too_long = format!("a://{}", "x".repeat(MAX_LENGTH - 3));
        assert_eq!(parse(&too_long), Err(UrlError::TooLong));
    }
}
