use std::fmt;

// Constants
pub const MAX_SIZE: usize = 64 * 1024;
pub const MAX_DEPTH: usize = 8;

// Value
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "a boolean",
            Self::Number(_) => "a number",
            Self::String(_) => "a string",
            Self::Array(_) => "an array",
            Self::Object(_) => "an object",
        }
    }
}

// Error
#[derive(Debug, PartialEq, Eq)]
pub struct JsonError {
    pub offset: usize,
    pub message: &'static str,
}

impl fmt::Display for JsonError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} at byte {}", self.message, self.offset)
    }
}

// Parsing
pub fn parse(input: &[u8]) -> Result<Value, JsonError> {
    if input.len() > MAX_SIZE {
        return Err(JsonError {
            offset: 0,
            message: "The file is larger than 64 KiB",
        });
    }
    if input.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(JsonError {
            offset: 0,
            message: "The file starts with a byte order mark",
        });
    }
    let text = std::str::from_utf8(input).map_err(|error| JsonError {
        offset: error.valid_up_to(),
        message: "The file is not valid UTF-8",
    })?;
    let mut parser = Parser {
        bytes: text.as_bytes(),
        position: 0,
    };
    parser.skip_whitespace();
    let value = parser.value(0)?;
    parser.skip_whitespace();
    if parser.position != parser.bytes.len() {
        return Err(parser.error("Unexpected content after the JSON value"));
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl Parser<'_> {
    fn error(&self, message: &'static str) -> JsonError {
        JsonError {
            offset: self.position,
            message,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.position += 1;
        }
    }

    fn expect(&mut self, byte: u8, message: &'static str) -> Result<(), JsonError> {
        if self.peek() == Some(byte) {
            self.position += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, JsonError> {
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Value::String),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.error("Unexpected character")),
            None => Err(self.error("Unexpected end of file")),
        }
    }

    fn literal(&mut self, text: &str, value: Value) -> Result<Value, JsonError> {
        if self.bytes[self.position..].starts_with(text.as_bytes()) {
            self.position += text.len();
            Ok(value)
        } else {
            Err(self.error("Unexpected character"))
        }
    }

    fn nested(&self, depth: usize) -> Result<usize, JsonError> {
        if depth >= MAX_DEPTH {
            Err(self.error("The file is nested too deeply"))
        } else {
            Ok(depth + 1)
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, JsonError> {
        let depth = self.nested(depth)?;
        self.position += 1;
        let mut members: Vec<(String, Value)> = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b'}') {
            self.position += 1;
            return Ok(Value::Object(members));
        }
        loop {
            self.skip_whitespace();
            if self.peek() != Some(b'"') {
                return Err(self.error("Expected a property name in double quotes"));
            }
            let key_position = self.position;
            let key = self.string()?;
            if members.iter().any(|(existing, _)| *existing == key) {
                self.position = key_position;
                return Err(self.error("Duplicate property name"));
            }
            self.skip_whitespace();
            self.expect(b':', "Expected ':' after the property name")?;
            self.skip_whitespace();
            let value = self.value(depth)?;
            members.push((key, value));
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b'}') => {
                    self.position += 1;
                    return Ok(Value::Object(members));
                }
                _ => return Err(self.error("Expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, JsonError> {
        let depth = self.nested(depth)?;
        self.position += 1;
        let mut items = Vec::new();
        self.skip_whitespace();
        if self.peek() == Some(b']') {
            self.position += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_whitespace();
            items.push(self.value(depth)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.position += 1,
                Some(b']') => {
                    self.position += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.error("Expected ',' or ']'")),
            }
        }
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        match self.peek() {
            Some(b'0') => self.position += 1,
            Some(b'1'..=b'9') => self.digits(),
            _ => return Err(self.error("Invalid number")),
        }
        if self.peek() == Some(b'.') {
            self.position += 1;
            self.required_digits()?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            self.required_digits()?;
        }
        if matches!(self.peek(), Some(b'0'..=b'9')) {
            return Err(self.error("Numbers must not have leading zeros"));
        }
        let text = String::from_utf8_lossy(&self.bytes[start..self.position]).into_owned();
        Ok(Value::Number(text))
    }

    fn digits(&mut self) {
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.position += 1;
        }
    }

    fn required_digits(&mut self) -> Result<(), JsonError> {
        if !matches!(self.peek(), Some(b'0'..=b'9')) {
            return Err(self.error("Invalid number"));
        }
        self.digits();
        Ok(())
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.position += 1;
        let mut text: Vec<u8> = Vec::new();
        loop {
            let Some(byte) = self.peek() else {
                return Err(self.error("Unterminated string"));
            };
            match byte {
                b'"' => {
                    self.position += 1;
                    return String::from_utf8(text)
                        .map_err(|_| self.error("Invalid text in string"));
                }
                b'\\' => {
                    self.position += 1;
                    let character = self.escape()?;
                    let mut buffer = [0; 4];
                    text.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                }
                0x00..=0x1F => return Err(self.error("Control character in string")),
                _ => {
                    text.push(byte);
                    self.position += 1;
                }
            }
        }
    }

    fn escape(&mut self) -> Result<char, JsonError> {
        let Some(byte) = self.peek() else {
            return Err(self.error("Unterminated string"));
        };
        self.position += 1;
        Ok(match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{0008}',
            b'f' => '\u{000C}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => return self.unicode_escape(),
            _ => {
                self.position -= 1;
                return Err(self.error("Invalid escape sequence"));
            }
        })
    }

    fn unicode_escape(&mut self) -> Result<char, JsonError> {
        let first = self.hex4()?;
        let code = match first {
            0xD800..=0xDBFF => {
                if !self.bytes[self.position..].starts_with(b"\\u") {
                    return Err(self.error("Unpaired surrogate in string"));
                }
                self.position += 2;
                let second = self.hex4()?;
                if !(0xDC00..=0xDFFF).contains(&second) {
                    return Err(self.error("Unpaired surrogate in string"));
                }
                0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
            }
            0xDC00..=0xDFFF => return Err(self.error("Unpaired surrogate in string")),
            _ => first,
        };
        char::from_u32(code).ok_or_else(|| self.error("Invalid escape sequence"))
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        let mut code = 0;
        for _ in 0..4 {
            let digit = self
                .peek()
                .and_then(|byte| char::from(byte).to_digit(16))
                .ok_or_else(|| self.error("Invalid escape sequence"))?;
            code = code * 16 + digit;
            self.position += 1;
        }
        Ok(code)
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn text(value: &str) -> Value {
        Value::String(value.to_owned())
    }

    fn message(input: &str) -> &'static str {
        parse(input.as_bytes()).unwrap_err().message
    }

    #[test]
    fn parses_nested_values() {
        let value = parse(br#" { "a" : [ 1 , -2.5e3 , true , false , null ] , "b" : "x" } "#);
        assert_eq!(
            value,
            Ok(Value::Object(vec![
                (
                    "a".to_owned(),
                    Value::Array(vec![
                        Value::Number("1".to_owned()),
                        Value::Number("-2.5e3".to_owned()),
                        Value::Bool(true),
                        Value::Bool(false),
                        Value::Null,
                    ])
                ),
                ("b".to_owned(), text("x")),
            ]))
        );
    }

    #[test]
    fn parses_empty_containers() {
        assert_eq!(parse(b"{}"), Ok(Value::Object(Vec::new())));
        assert_eq!(parse(b"[ ]"), Ok(Value::Array(Vec::new())));
    }

    #[test]
    fn decodes_escapes() {
        assert_eq!(
            parse(br#""a\"\\\/\b\f\n\r\t\u00e4\ud83d\ude00""#),
            Ok(text("a\"\\/\u{8}\u{c}\n\r\tä😀"))
        );
        assert_eq!(parse("\"Grüße\"".as_bytes()), Ok(text("Grüße")));
    }

    #[test]
    fn rejects_duplicate_keys() {
        assert_eq!(message(r#"{"a":1,"a":2}"#), "Duplicate property name");
    }

    #[test]
    fn rejects_trailing_commas() {
        assert_eq!(message("[1,]"), "Unexpected character");
        assert_eq!(
            message(r#"{"a":1,}"#),
            "Expected a property name in double quotes"
        );
    }

    #[test]
    fn rejects_comments() {
        assert_eq!(message("// x\n{}"), "Unexpected character");
        assert_eq!(
            message("{/* x */}"),
            "Expected a property name in double quotes"
        );
    }

    #[test]
    fn rejects_trailing_content() {
        assert_eq!(message("{} {}"), "Unexpected content after the JSON value");
        assert_eq!(message("1 2"), "Unexpected content after the JSON value");
    }

    #[test]
    fn rejects_invalid_numbers() {
        for input in ["01", "-", "1.", ".5", "1e", "+1", "0x10", "1e+"] {
            assert!(parse(input.as_bytes()).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_invalid_strings() {
        for input in [
            "\"a",
            "\"a\nb\"",
            "\"\\x\"",
            "\"\\u12\"",
            "\"\\ud800\"",
            "\"\\ud800\\u0041\"",
            "\"\\udc00\"",
            "'a'",
        ] {
            assert!(parse(input.as_bytes()).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_invalid_literals() {
        for input in ["tru", "True", "nul", "NaN", "Infinity", "undefined", ""] {
            assert!(parse(input.as_bytes()).is_err(), "{input}");
        }
    }

    #[test]
    fn rejects_byte_order_mark_and_invalid_utf8() {
        assert_eq!(
            parse(b"\xEF\xBB\xBF{}").unwrap_err().message,
            "The file starts with a byte order mark"
        );
        assert_eq!(
            parse(b"\"\xFF\"").unwrap_err().message,
            "The file is not valid UTF-8"
        );
    }

    #[test]
    fn limits_depth() {
        let allowed = "[".repeat(MAX_DEPTH) + &"]".repeat(MAX_DEPTH);
        assert!(parse(allowed.as_bytes()).is_ok());
        let too_deep = "[".repeat(MAX_DEPTH + 1) + &"]".repeat(MAX_DEPTH + 1);
        assert_eq!(message(&too_deep), "The file is nested too deeply");
    }

    #[test]
    fn limits_size() {
        let large = format!("\"{}\"", "a".repeat(MAX_SIZE));
        assert_eq!(message(&large), "The file is larger than 64 KiB");
    }

    #[test]
    fn reports_the_offset() {
        assert_eq!(parse(b"[1, x]").unwrap_err().offset, 4);
    }
}
