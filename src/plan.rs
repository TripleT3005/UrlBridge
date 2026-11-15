use std::fmt;

use crate::config::{Config, Protocol, Segment};
use crate::url::{self, UrlError};

// Plan
#[derive(Debug, PartialEq, Eq)]
pub struct Plan<'a> {
    pub protocol: &'a Protocol,
    pub arguments: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Rejection {
    Url(UrlError),
    UnknownScheme(String),
    ValueCount { expected: usize, actual: usize },
    Value { name: String, problem: &'static str },
}

impl fmt::Display for Rejection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Url(error) => error.fmt(formatter),
            Self::UnknownScheme(scheme) => {
                write!(formatter, "The protocol \"{scheme}\" is not configured.")
            }
            Self::ValueCount { expected, actual } => write!(
                formatter,
                "The URL contains {actual} value(s), but the protocol expects {expected}."
            ),
            Self::Value { name, problem } => {
                write!(formatter, "The value for \"{name}\" {problem}.")
            }
        }
    }
}

impl<'a> Plan<'a> {
    pub fn new(config: &'a Config, url: &str) -> Result<Self, Rejection> {
        let request = url::parse(url).map_err(Rejection::Url)?;
        let protocol = config
            .protocol(&request.scheme)
            .ok_or(Rejection::UnknownScheme(request.scheme))?;
        if request.values.len() != protocol.parameters.len() {
            return Err(Rejection::ValueCount {
                expected: protocol.parameters.len(),
                actual: request.values.len(),
            });
        }
        for (parameter, value) in protocol.parameters.iter().zip(&request.values) {
            parameter
                .kind
                .check(value)
                .map_err(|problem| Rejection::Value {
                    name: parameter.name.clone(),
                    problem,
                })?;
        }
        let arguments = protocol
            .arguments
            .iter()
            .map(|segments| {
                segments
                    .iter()
                    .map(|segment| match segment {
                        Segment::Text(text) => text.as_str(),
                        Segment::Parameter(index) => request.values[*index].as_str(),
                    })
                    .collect()
            })
            .collect();
        Ok(Self {
            protocol,
            arguments,
        })
    }

    pub fn command_line(&self) -> String {
        let mut line = quote(&self.protocol.program.path);
        for argument in &self.arguments {
            line.push(' ');
            line.push_str(&quote(argument));
        }
        line
    }

    pub fn argument_line(&self) -> String {
        let arguments: Vec<String> = self
            .arguments
            .iter()
            .map(|argument| quote(argument))
            .collect();
        arguments.join(" ")
    }
}

// Quoting
pub fn quote(argument: &str) -> String {
    if !argument.is_empty() && !argument.contains([' ', '\t', '\n', '\u{b}', '"']) {
        return argument.to_owned();
    }
    let mut quoted = String::from('"');
    let mut backslashes = 0;
    for character in argument.chars() {
        match character {
            '\\' => backslashes += 1,
            '"' => {
                quoted.extend(std::iter::repeat_n('\\', backslashes + 1));
                backslashes = 0;
            }
            _ => backslashes = 0,
        }
        quoted.push(character);
    }
    quoted.extend(std::iter::repeat_n('\\', backslashes));
    quoted.push('"');
    quoted
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    const CONFIG: &str = r#"{"protocols":[
        {"scheme":"bridge-rdp","name":"Remote Desktop","program":"C:\\Windows\\System32\\mstsc.exe",
         "arguments":["/v:{host}:{port}","/f"],
         "parameters":[{"name":"host","type":"host"},{"name":"port","type":"integer","min":1,"max":65535}]},
        {"scheme":"bridge-tool","name":"Tool","program":"C:\\Program Files\\My Tool\\tool.exe",
         "arguments":["--mode","{mode}","--title","Remote \"control\""],
         "parameters":[{"name":"mode","type":"choice","values":["view","control"]}]},
        {"scheme":"bridge-fixed","name":"Fixed","program":"C:\\a.exe","arguments":[],"parameters":[]}
    ]}"#;

    fn config() -> Config {
        config::parse(CONFIG.as_bytes()).unwrap()
    }

    fn arguments(url: &str) -> Vec<String> {
        Plan::new(&config(), url).unwrap().arguments
    }

    fn rejection(url: &str) -> Rejection {
        Plan::new(&config(), url).unwrap_err()
    }

    #[test]
    fn fills_placeholders() {
        assert_eq!(
            arguments("bridge-rdp://pc0042/3389"),
            ["/v:pc0042:3389", "/f"]
        );
        assert_eq!(
            arguments("BRIDGE-RDP://10.0.0.1/3389/"),
            ["/v:10.0.0.1:3389", "/f"]
        );
        assert_eq!(arguments("bridge-fixed://"), Vec::<String>::new());
    }

    #[test]
    fn builds_the_command_line() {
        let config = config();
        let plan = Plan::new(&config, "bridge-tool://control").unwrap();
        assert_eq!(
            plan.command_line(),
            r#""C:\Program Files\My Tool\tool.exe" --mode control --title "Remote \"control\"""#
        );
        assert_eq!(
            plan.argument_line(),
            r#"--mode control --title "Remote \"control\"""#
        );
    }

    #[test]
    fn rejects_unknown_schemes() {
        assert_eq!(
            rejection("other://pc0042"),
            Rejection::UnknownScheme("other".to_owned())
        );
    }

    #[test]
    fn rejects_a_wrong_number_of_values() {
        assert_eq!(
            rejection("bridge-rdp://pc0042"),
            Rejection::ValueCount {
                expected: 2,
                actual: 1
            }
        );
        assert_eq!(
            rejection("bridge-rdp://pc0042/3389/x"),
            Rejection::ValueCount {
                expected: 2,
                actual: 3
            }
        );
        assert_eq!(
            rejection("bridge-fixed://x"),
            Rejection::ValueCount {
                expected: 0,
                actual: 1
            }
        );
    }

    #[test]
    fn rejects_invalid_values() {
        for url in [
            "bridge-rdp://pc_1/3389",
            "bridge-rdp://pc0042/0",
            "bridge-rdp://pc0042/65536",
            "bridge-rdp://pc0042/33%2089",
            "bridge-tool://admin",
            "bridge-tool://Control",
        ] {
            assert!(matches!(rejection(url), Rejection::Value { .. }), "{url}");
        }
    }

    #[test]
    fn rejects_injection_attempts() {
        for url in [
            "bridge-rdp://pc0042%20/admin/3389",
            "bridge-rdp://pc0042%22%20/admin/3389",
            "bridge-rdp://pc0042%20%2Fadmin/3389",
            "bridge-rdp://pc0042%22%20%2Fadmin%20%22/3389",
            "bridge-rdp://pc0042%00/3389",
            "bridge-rdp://pc0042%0A/3389",
            "bridge-rdp://-v/3389",
            "bridge-rdp://%2Dv/3389",
            "bridge-rdp://..%5C..%5Cx/3389",
            "bridge-rdp://pc0042%26calc/3389",
            "bridge-rdp://%25COMPUTERNAME%25/3389",
            "bridge-rdp://pc0042/3389?x",
            "bridge-rdp://pc0042/3389#x",
            "bridge-rdp://user@pc0042/3389",
            "bridge-rdp://pc0042 /admin/3389",
            "bridge-rdp://pc0042\" /admin/3389",
            "bridge-tool://control%20--other",
        ] {
            assert!(Plan::new(&config(), url).is_err(), "{url}");
        }
    }

    #[test]
    fn quotes_like_command_line_to_argv() {
        assert_eq!(quote("plain"), "plain");
        assert_eq!(quote(r"C:\dir\file.exe"), r"C:\dir\file.exe");
        assert_eq!(quote(""), r#""""#);
        assert_eq!(quote("a b"), r#""a b""#);
        assert_eq!(quote(r#"a"b"#), r#""a\"b""#);
        assert_eq!(quote(r#"a\"b"#), r#""a\\\"b""#);
        assert_eq!(quote(r"a b\"), r#""a b\\""#);
        assert_eq!(quote(r"a\\b c"), r#""a\\b c""#);
        assert_eq!(quote(r"a\b"), r"a\b");
    }
}
