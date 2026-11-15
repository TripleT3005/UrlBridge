use crate::json::{self, Value};
use crate::value::{self, Kind};

// Constants
const MAX_PROTOCOLS: usize = 64;
const MAX_ARGUMENTS: usize = 32;
const MAX_PARAMETERS: usize = 8;
const MAX_CHOICES: usize = 64;
const MAX_NAME_LENGTH: usize = 64;
const MAX_ARGUMENT_LENGTH: usize = 1024;
const MAX_PATH_LENGTH: usize = 259;
const MAX_PARAMETER_NAME_LENGTH: usize = 32;
const MIN_SCHEME_LENGTH: usize = 3;
const MAX_SCHEME_LENGTH: usize = 64;
const PROGRAM_EXTENSION: &str = ".exe";
const UNC_PREFIX: &str = r"\\";
const RESERVED_SCHEME_PREFIX: &str = "ms-";
const UNSUPPORTED_SCHEME_PREFIX: &str = "url";
const RESERVED_SCHEMES: &[&str] = &[
    "about",
    "blob",
    "callto",
    "chrome",
    "data",
    "edge",
    "file",
    "ftp",
    "ftps",
    "http",
    "https",
    "javascript",
    "ldap",
    "ldaps",
    "mailto",
    "microsoft-edge",
    "news",
    "nntp",
    "read",
    "res",
    "search",
    "search-ms",
    "sftp",
    "shell",
    "sms",
    "smb",
    "ssh",
    "tel",
    "telnet",
    "vbscript",
    "view-source",
    "ws",
    "wss",
];

// Configuration
#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    pub protocols: Vec<Protocol>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Protocol {
    pub scheme: String,
    pub name: String,
    pub program: Program,
    pub arguments: Vec<Vec<Segment>>,
    pub parameters: Vec<Parameter>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Program {
    pub path: String,
    pub location: Location,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Local,
    Network,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Segment {
    Text(String),
    Parameter(usize),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Parameter {
    pub name: String,
    pub kind: Kind,
}

impl Config {
    pub fn protocol(&self, scheme: &str) -> Option<&Protocol> {
        self.protocols
            .iter()
            .find(|protocol| protocol.scheme == scheme)
    }
}

// Parsing
pub fn parse(input: &[u8]) -> Result<Config, Vec<String>> {
    let root = json::parse(input).map_err(|error| vec![format!("Invalid JSON: {error}.")])?;
    let mut errors = Vec::new();
    let protocols = parse_root(&root, &mut errors);
    if errors.is_empty() {
        Ok(Config { protocols })
    } else {
        Err(errors)
    }
}

fn parse_root(root: &Value, errors: &mut Vec<String>) -> Vec<Protocol> {
    let Some(fields) = Fields::new(root, "The configuration", &["protocols"], errors) else {
        return Vec::new();
    };
    let Some(items) = fields.array("protocols", MAX_PROTOCOLS, errors) else {
        return Vec::new();
    };
    let mut protocols: Vec<Protocol> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let context = format!("protocols[{index}]");
        let Some(protocol) = parse_protocol(item, &context, errors) else {
            continue;
        };
        if protocols
            .iter()
            .any(|existing| existing.scheme == protocol.scheme)
        {
            errors.push(format!(
                "{context}: the scheme \"{}\" is used more than once.",
                protocol.scheme
            ));
            continue;
        }
        protocols.push(protocol);
    }
    protocols
}

fn parse_protocol(value: &Value, context: &str, errors: &mut Vec<String>) -> Option<Protocol> {
    let fields = Fields::new(
        value,
        context,
        &["scheme", "name", "program", "arguments", "parameters"],
        errors,
    )?;
    let before = errors.len();
    let scheme = fields.string("scheme", errors).and_then(|scheme| {
        check(scheme_problem(scheme), &fields.path("scheme"), errors).then(|| scheme.to_owned())
    });
    let name = fields.string("name", errors).and_then(|name| {
        check(name_problem(name), &fields.path("name"), errors).then(|| name.to_owned())
    });
    let program = fields
        .string("program", errors)
        .and_then(|path| match parse_program(path) {
            Ok(program) => Some(program),
            Err(problem) => {
                errors.push(format!("{}: {problem}.", fields.path("program")));
                None
            }
        });
    let parameters = parse_parameters(&fields, errors);
    let arguments = parameters
        .as_ref()
        .and_then(|parameters| parse_arguments(&fields, parameters, errors));
    if errors.len() > before {
        return None;
    }
    Some(Protocol {
        scheme: scheme?,
        name: name?,
        program: program?,
        arguments: arguments?,
        parameters: parameters?,
    })
}

fn check(problem: Option<&'static str>, path: &str, errors: &mut Vec<String>) -> bool {
    if let Some(problem) = problem {
        errors.push(format!("{path}: {problem}."));
    }
    problem.is_none()
}

// Scheme and name
fn scheme_problem(scheme: &str) -> Option<&'static str> {
    if !(MIN_SCHEME_LENGTH..=MAX_SCHEME_LENGTH).contains(&scheme.len()) {
        return Some("must have 3 to 64 characters");
    }
    let valid = scheme.starts_with(|first: char| first.is_ascii_lowercase())
        && scheme.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.')
        });
    if !valid {
        return Some(
            "must start with a lowercase letter and contain only lowercase letters, digits, '+', '-' and '.'",
        );
    }
    if scheme.starts_with(RESERVED_SCHEME_PREFIX) || RESERVED_SCHEMES.contains(&scheme) {
        return Some("is a reserved scheme");
    }
    if scheme.starts_with(UNSUPPORTED_SCHEME_PREFIX) {
        return Some("must not start with 'url', because Windows does not open such protocols");
    }
    None
}

fn name_problem(name: &str) -> Option<&'static str> {
    if name.is_empty() || name.chars().count() > MAX_NAME_LENGTH {
        return Some("must have 1 to 64 characters");
    }
    if name.chars().any(char::is_control) || name.trim() != name {
        return Some("must not contain control characters or surrounding spaces");
    }
    None
}

// Program
pub fn parse_program(path: &str) -> Result<Program, &'static str> {
    if path.len() > MAX_PATH_LENGTH {
        return Err("is longer than 259 characters");
    }
    if path.chars().any(char::is_control) {
        return Err("contains control characters");
    }
    if path.contains('/') {
        return Err("must use backslashes");
    }
    if path.contains('%') {
        return Err("must not contain environment variables");
    }
    let (location, components) = if let Some(rest) = path.strip_prefix(UNC_PREFIX) {
        let components: Vec<&str> = rest.split('\\').collect();
        if components.len() < 3 || matches!(components[0], "?" | "." | "") {
            return Err(
                "must be an absolute path such as C:\\Folder\\Tool.exe or \\\\server\\share\\Tool.exe",
            );
        }
        (Location::Network, components)
    } else {
        let bytes = path.as_bytes();
        let has_drive = bytes.len() > 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && bytes[2] == b'\\';
        if !has_drive {
            return Err(
                "must be an absolute path such as C:\\Folder\\Tool.exe or \\\\server\\share\\Tool.exe",
            );
        }
        (Location::Local, path[3..].split('\\').collect())
    };
    if !components.iter().all(|component| is_component(component)) {
        return Err(
            "contains an empty, relative or invalid path component ('..', trailing dot or space, or one of < > : \" | ? *)",
        );
    }
    let is_program = path
        .get(path.len().saturating_sub(PROGRAM_EXTENSION.len())..)
        .is_some_and(|extension| extension.eq_ignore_ascii_case(PROGRAM_EXTENSION));
    if !is_program {
        return Err("must be an .exe file");
    }
    Ok(Program {
        path: path.to_owned(),
        location,
    })
}

fn is_component(component: &str) -> bool {
    !component.is_empty()
        && component != "."
        && component != ".."
        && !component.ends_with(['.', ' '])
        && !component.contains(['<', '>', ':', '"', '|', '?', '*'])
}

// Parameters
fn parse_parameters(fields: &Fields, errors: &mut Vec<String>) -> Option<Vec<Parameter>> {
    let items = fields.array("parameters", MAX_PARAMETERS, errors)?;
    let before = errors.len();
    let mut parameters: Vec<Parameter> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let context = format!("{}[{index}]", fields.path("parameters"));
        let Some(parameter) = parse_parameter(item, &context, errors) else {
            continue;
        };
        if parameters
            .iter()
            .any(|existing| existing.name == parameter.name)
        {
            errors.push(format!(
                "{context}: the name \"{}\" is used more than once.",
                parameter.name
            ));
            continue;
        }
        parameters.push(parameter);
    }
    (errors.len() == before).then_some(parameters)
}

fn parse_parameter(value: &Value, context: &str, errors: &mut Vec<String>) -> Option<Parameter> {
    let type_name = match value {
        Value::Object(members) => {
            members
                .iter()
                .find(|(key, _)| key == "type")
                .and_then(|(_, value)| match value {
                    Value::String(text) => Some(text.as_str()),
                    _ => None,
                })
        }
        _ => None,
    };
    let allowed: &[&str] = match type_name {
        Some("integer") => &["name", "type", "min", "max"],
        Some("token") => &["name", "type", "maxLength"],
        Some("choice") => &["name", "type", "values"],
        _ => &["name", "type"],
    };
    let fields = Fields::new(value, context, allowed, errors)?;
    let before = errors.len();
    let name = fields.string("name", errors).and_then(|name| {
        check(parameter_name_problem(name), &fields.path("name"), errors).then(|| name.to_owned())
    });
    let kind = fields
        .string("type", errors)
        .and_then(|type_name| parse_kind(type_name, &fields, errors));
    if errors.len() > before {
        return None;
    }
    Some(Parameter {
        name: name?,
        kind: kind?,
    })
}

fn parameter_name_problem(name: &str) -> Option<&'static str> {
    let valid = (1..=MAX_PARAMETER_NAME_LENGTH).contains(&name.len())
        && name.starts_with(|first: char| first.is_ascii_alphabetic())
        && name.bytes().all(|byte| byte.is_ascii_alphanumeric());
    (!valid).then_some("must start with a letter and contain only 1 to 32 letters and digits")
}

fn parse_kind(type_name: &str, fields: &Fields, errors: &mut Vec<String>) -> Option<Kind> {
    match type_name {
        "hostname" => Some(Kind::Hostname),
        "ipv4" => Some(Kind::Ipv4),
        "host" => Some(Kind::Host),
        "integer" => {
            let min = fields.integer("min", errors);
            let max = fields.integer("max", errors);
            let (min, max) = (min?, max?);
            if min > max {
                errors.push(format!(
                    "{}: must not be greater than \"max\".",
                    fields.path("min")
                ));
                return None;
            }
            Some(Kind::Integer { min, max })
        }
        "token" => {
            let max_length = fields.integer("maxLength", errors)?;
            let max_length = usize::try_from(max_length).unwrap_or(usize::MAX);
            if !(1..=value::MAX_VALUE_LENGTH).contains(&max_length) {
                errors.push(format!(
                    "{}: must be between 1 and 256.",
                    fields.path("maxLength")
                ));
                return None;
            }
            Some(Kind::Token { max_length })
        }
        "choice" => parse_choices(fields, errors).map(|values| Kind::Choice { values }),
        _ => {
            errors.push(format!(
                "{}: must be one of hostname, ipv4, host, integer, token, choice.",
                fields.path("type")
            ));
            None
        }
    }
}

fn parse_choices(fields: &Fields, errors: &mut Vec<String>) -> Option<Vec<String>> {
    let items = fields.array("values", MAX_CHOICES, errors)?;
    let path = fields.path("values");
    if items.is_empty() {
        errors.push(format!("{path}: must not be empty."));
        return None;
    }
    let mut values: Vec<String> = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Value::String(text) = item else {
            errors.push(format!("{path}[{index}]: must be a string."));
            return None;
        };
        if !value::is_safe(text) {
            errors.push(format!(
                "{path}[{index}]: must contain only letters, digits, '-', '_' and '.', must not start with '-' and must have 1 to 256 characters."
            ));
            return None;
        }
        if values.contains(text) {
            errors.push(format!("{path}[{index}]: is listed more than once."));
            return None;
        }
        values.push(text.clone());
    }
    Some(values)
}

// Arguments
fn parse_arguments(
    fields: &Fields,
    parameters: &[Parameter],
    errors: &mut Vec<String>,
) -> Option<Vec<Vec<Segment>>> {
    let items = fields.array("arguments", MAX_ARGUMENTS, errors)?;
    let before = errors.len();
    let mut arguments = Vec::new();
    let mut used = vec![false; parameters.len()];
    for (index, item) in items.iter().enumerate() {
        let path = format!("{}[{index}]", fields.path("arguments"));
        let Value::String(text) = item else {
            errors.push(format!("{path}: must be a string."));
            continue;
        };
        match parse_template(text, parameters) {
            Ok(segments) => {
                for segment in &segments {
                    if let Segment::Parameter(parameter) = segment {
                        used[*parameter] = true;
                    }
                }
                arguments.push(segments);
            }
            Err(problem) => errors.push(format!("{path}: {problem}.")),
        }
    }
    if errors.len() == before {
        for (parameter, used) in parameters.iter().zip(&used) {
            if !used {
                errors.push(format!(
                    "{}: the parameter \"{}\" is not used in any argument.",
                    fields.path("arguments"),
                    parameter.name
                ));
            }
        }
    }
    (errors.len() == before).then_some(arguments)
}

fn parse_template(text: &str, parameters: &[Parameter]) -> Result<Vec<Segment>, &'static str> {
    if text.is_empty() || text.chars().count() > MAX_ARGUMENT_LENGTH {
        return Err("must have 1 to 1024 characters");
    }
    if text.chars().any(char::is_control) {
        return Err("contains control characters");
    }
    let mut segments = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(['{', '}']) {
        if rest[start..].starts_with('}') {
            return Err("contains a '}' without a matching '{'");
        }
        if start > 0 {
            segments.push(Segment::Text(rest[..start].to_owned()));
        }
        let after = &rest[start + 1..];
        let end = after
            .find(['{', '}'])
            .filter(|&end| after[end..].starts_with('}'))
            .ok_or("contains a '{' without a matching '}'")?;
        let name = &after[..end];
        let parameter = parameters
            .iter()
            .position(|parameter| parameter.name == name)
            .ok_or("uses a placeholder that is not declared in \"parameters\"")?;
        segments.push(Segment::Parameter(parameter));
        rest = &after[end + 1..];
    }
    if !rest.is_empty() {
        segments.push(Segment::Text(rest.to_owned()));
    }
    Ok(segments)
}

// Fields
struct Fields<'a> {
    context: &'a str,
    members: &'a [(String, Value)],
}

impl<'a> Fields<'a> {
    fn new(
        value: &'a Value,
        context: &'a str,
        allowed: &[&str],
        errors: &mut Vec<String>,
    ) -> Option<Self> {
        let Value::Object(members) = value else {
            errors.push(format!(
                "{context}: must be an object, but is {}.",
                value.kind()
            ));
            return None;
        };
        for (key, _) in members {
            if !allowed.contains(&key.as_str()) {
                errors.push(format!("{context}: unknown field \"{key}\"."));
            }
        }
        Some(Self { context, members })
    }

    fn path(&self, name: &str) -> String {
        format!("{}.{name}", self.context)
    }

    fn get(&self, name: &str, errors: &mut Vec<String>) -> Option<&'a Value> {
        let value = self
            .members
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value);
        if value.is_none() {
            errors.push(format!("{}: missing field \"{name}\".", self.context));
        }
        value
    }

    fn string(&self, name: &str, errors: &mut Vec<String>) -> Option<&'a str> {
        match self.get(name, errors)? {
            Value::String(text) => Some(text),
            other => {
                errors.push(format!(
                    "{}: must be a string, but is {}.",
                    self.path(name),
                    other.kind()
                ));
                None
            }
        }
    }

    fn integer(&self, name: &str, errors: &mut Vec<String>) -> Option<u32> {
        let number = match self.get(name, errors)? {
            Value::Number(text) => value::parse_integer(text),
            _ => None,
        };
        if number.is_none() {
            errors.push(format!(
                "{}: must be a whole number between 0 and 4294967295.",
                self.path(name)
            ));
        }
        number
    }

    fn array(&self, name: &str, max: usize, errors: &mut Vec<String>) -> Option<&'a [Value]> {
        match self.get(name, errors)? {
            Value::Array(items) if items.len() <= max => Some(items),
            Value::Array(_) => {
                errors.push(format!(
                    "{}: must not have more than {max} entries.",
                    self.path(name)
                ));
                None
            }
            other => {
                errors.push(format!(
                    "{}: must be an array, but is {}.",
                    self.path(name),
                    other.kind()
                ));
                None
            }
        }
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../config/UrlBridge.example.json");

    fn protocol(body: &str) -> Result<Config, Vec<String>> {
        parse(format!(r#"{{"protocols":[{body}]}}"#).as_bytes())
    }

    fn with(field: &str, value: &str) -> Result<Config, Vec<String>> {
        let mut fields = vec![
            ("scheme", r#""bridge-test""#),
            ("name", r#""Test""#),
            ("program", r#""C:\\Tools\\Tool.exe""#),
            ("arguments", r#"["{host}"]"#),
            ("parameters", r#"[{"name":"host","type":"host"}]"#),
        ];
        fields.retain(|(name, _)| *name != field);
        if !value.is_empty() {
            fields.push((field, value));
        }
        let body: Vec<String> = fields
            .iter()
            .map(|(name, value)| format!(r#""{name}":{value}"#))
            .collect();
        protocol(&format!("{{{}}}", body.join(",")))
    }

    fn error(result: Result<Config, Vec<String>>) -> String {
        result.unwrap_err().join("\n")
    }

    fn parameter(definition: &str) -> Result<Config, Vec<String>> {
        with("parameters", &format!("[{definition}]"))
    }

    #[test]
    fn parses_the_example() {
        let config = parse(EXAMPLE.as_bytes()).unwrap();
        assert_eq!(config.protocols.len(), 1);
        let protocol = config.protocol("bridge-rdp").unwrap();
        assert_eq!(protocol.name, "Remote Desktop");
        assert_eq!(protocol.program.path, r"C:\Windows\System32\mstsc.exe");
        assert_eq!(protocol.program.location, Location::Local);
        assert_eq!(
            protocol.arguments,
            [vec![Segment::Text("/v:".to_owned()), Segment::Parameter(0)]]
        );
        assert_eq!(
            protocol.parameters,
            [Parameter {
                name: "host".to_owned(),
                kind: Kind::Host
            }]
        );
    }

    #[test]
    fn accepts_an_empty_protocol_list() {
        assert_eq!(
            parse(br#"{"protocols":[]}"#),
            Ok(Config {
                protocols: Vec::new()
            })
        );
    }

    #[test]
    fn accepts_a_protocol_without_parameters() {
        let config = protocol(
            r#"{"scheme":"abc","name":"A","program":"C:\\a.exe","arguments":[],"parameters":[]}"#,
        )
        .unwrap();
        assert_eq!(config.protocols[0].arguments, Vec::<Vec<Segment>>::new());
    }

    #[test]
    fn reports_invalid_json() {
        assert!(error(parse(b"{")).starts_with("Invalid JSON:"));
        assert!(error(parse(br#"{"protocols":[],"protocols":[]}"#)).contains("Duplicate"));
    }

    #[test]
    fn rejects_a_wrong_root() {
        assert!(error(parse(b"[]")).contains("must be an object"));
        assert!(error(parse(b"{}")).contains("missing field \"protocols\""));
        assert!(error(parse(br#"{"protocols":{}}"#)).contains("must be an array"));
        assert!(
            error(parse(br#"{"protocols":[],"trustedSids":[]}"#))
                .contains("unknown field \"trustedSids\"")
        );
    }

    #[test]
    fn rejects_unknown_and_missing_fields() {
        assert!(error(with("shell", "true")).contains("unknown field \"shell\""));
        for field in ["scheme", "name", "program", "arguments", "parameters"] {
            assert!(
                error(with(field, "")).contains(&format!("missing field \"{field}\"")),
                "{field}"
            );
        }
    }

    #[test]
    fn rejects_wrong_types() {
        assert!(error(with("scheme", "1")).contains("must be a string"));
        assert!(error(with("arguments", r#""x""#)).contains("must be an array"));
        assert!(error(with("arguments", "[1]")).contains("must be a string"));
        assert!(error(with("parameters", "[1]")).contains("must be an object"));
    }

    #[test]
    fn checks_schemes() {
        for scheme in [
            "ab",
            "Abc",
            "1abc",
            "a_b",
            "a b",
            "a:b",
            "http",
            "https",
            "file",
            "mailto",
            "ms-settings",
            "search-ms",
            "javascript",
            "url",
            "urlbridge-rdp",
            "url-test",
            &"a".repeat(65),
        ] {
            assert!(
                with("scheme", &format!(r#""{scheme}""#)).is_err(),
                "{scheme}"
            );
        }
        for scheme in [
            "abc",
            "bridge-rdp",
            "a.b+c-1",
            "curl",
            "ur-l",
            &"a".repeat(64),
        ] {
            assert!(
                with("scheme", &format!(r#""{scheme}""#)).is_ok(),
                "{scheme}"
            );
        }
    }

    #[test]
    fn rejects_duplicate_schemes() {
        let one =
            r#"{"scheme":"abc","name":"A","program":"C:\\a.exe","arguments":[],"parameters":[]}"#;
        assert!(error(protocol(&format!("{one},{one}"))).contains("used more than once"));
    }

    #[test]
    fn checks_names() {
        for name in [r#""""#, r#"" A""#, r#""A\n""#, r#""A\u0007""#] {
            assert!(with("name", name).is_err(), "{name}");
        }
        assert!(with("name", &format!(r#""{}""#, "ä".repeat(64))).is_ok());
        assert!(with("name", &format!(r#""{}""#, "a".repeat(65))).is_err());
    }

    #[test]
    fn checks_program_paths() {
        for path in [
            r"C:\Windows\System32\mstsc.exe",
            r"c:\a.EXE",
            r"D:\My Tools\Remote Control\rc.exe",
            r"\\server\share\tool.exe",
            r"\\server.example.com\share$\dir\tool.exe",
        ] {
            assert!(parse_program(path).is_ok(), "{path}");
        }
        for path in [
            "",
            "mstsc.exe",
            r"Tools\tool.exe",
            r"\Tools\tool.exe",
            r"C:tool.exe",
            r"C:\",
            r"C:\Tools\..\tool.exe",
            r"C:\Tools\.\tool.exe",
            r"C:\Tools\\tool.exe",
            r"C:\Tools\tool.exe ",
            r"C:\Tools\tool.exe.",
            r"C:\Tools \tool.exe",
            r"C:\Tools\tool.exe:stream",
            r"C:\Tools\tool.exe:stream.exe",
            r"C:\Tools\to*l.exe",
            r"C:/Tools/tool.exe",
            r"%ProgramFiles%\Tool\tool.exe",
            r"C:\%TEMP%\tool.exe",
            r"C:\Tools\tool.bat",
            r"C:\Tools\tool.cmd",
            r"C:\Tools\tool.ps1",
            r"C:\Tools\tool",
            r"C:\Tools\exe",
            r"\\?\C:\Tools\tool.exe",
            r"\\.\C:\Tools\tool.exe",
            r"\\server\tool.exe",
            r"\\server\share",
            r"\\\share\tool.exe",
            "C:\\Tools\\to\tol.exe",
        ] {
            assert!(parse_program(path).is_err(), "{path}");
        }
        assert!(parse_program(&format!(r"C:\{}.exe", "a".repeat(260))).is_err());
        assert_eq!(
            parse_program(r"\\server\share\tool.exe").unwrap().location,
            Location::Network
        );
    }

    #[test]
    fn checks_parameter_names() {
        for name in ["", "1a", "a-b", "a b", "ä", &"a".repeat(33)] {
            assert!(
                parameter(&format!(r#"{{"name":"{name}","type":"host"}}"#)).is_err(),
                "{name}"
            );
        }
        let duplicate = r#"{"name":"host","type":"host"},{"name":"host","type":"ipv4"}"#;
        assert!(error(parameter(duplicate)).contains("used more than once"));
    }

    #[test]
    fn parses_every_parameter_type() {
        let kind = |definition: &str| {
            parameter(definition).unwrap().protocols[0].parameters[0]
                .kind
                .clone()
        };
        assert_eq!(kind(r#"{"name":"host","type":"hostname"}"#), Kind::Hostname);
        assert_eq!(kind(r#"{"name":"host","type":"ipv4"}"#), Kind::Ipv4);
        assert_eq!(
            kind(r#"{"name":"host","type":"integer","min":1,"max":65535}"#),
            Kind::Integer { min: 1, max: 65535 }
        );
        assert_eq!(
            kind(r#"{"name":"host","type":"token","maxLength":16}"#),
            Kind::Token { max_length: 16 }
        );
        assert_eq!(
            kind(r#"{"name":"host","type":"choice","values":["a","b"]}"#),
            Kind::Choice {
                values: vec!["a".to_owned(), "b".to_owned()]
            }
        );
    }

    #[test]
    fn rejects_invalid_parameter_types() {
        for definition in [
            r#"{"name":"host","type":"regex"}"#,
            r#"{"name":"host","type":"Host"}"#,
            r#"{"name":"host","type":"host","min":1}"#,
            r#"{"name":"host","type":"host","values":["a"]}"#,
            r#"{"name":"host","type":"integer"}"#,
            r#"{"name":"host","type":"integer","min":1}"#,
            r#"{"name":"host","type":"integer","min":2,"max":1}"#,
            r#"{"name":"host","type":"integer","min":-1,"max":1}"#,
            r#"{"name":"host","type":"integer","min":1.5,"max":2}"#,
            r#"{"name":"host","type":"integer","min":"1","max":2}"#,
            r#"{"name":"host","type":"integer","min":0,"max":4294967296}"#,
            r#"{"name":"host","type":"integer","min":0,"max":1,"maxLength":1}"#,
            r#"{"name":"host","type":"token"}"#,
            r#"{"name":"host","type":"token","maxLength":0}"#,
            r#"{"name":"host","type":"token","maxLength":257}"#,
            r#"{"name":"host","type":"choice"}"#,
            r#"{"name":"host","type":"choice","values":[]}"#,
            r#"{"name":"host","type":"choice","values":[1]}"#,
            r#"{"name":"host","type":"choice","values":["a","a"]}"#,
            r#"{"name":"host","type":"choice","values":["a b"]}"#,
            r#"{"name":"host","type":"choice","values":["-a"]}"#,
            r#"{"name":"host","type":"choice","values":["a\"b"]}"#,
            r#"{"name":"host","type":"choice","values":[""]}"#,
        ] {
            assert!(parameter(definition).is_err(), "{definition}");
        }
    }

    #[test]
    fn parses_argument_templates() {
        let config = with("arguments", r#"["/v:{host}","--fixed","{host}:{host}"]"#).unwrap();
        assert_eq!(
            config.protocols[0].arguments,
            [
                vec![Segment::Text("/v:".to_owned()), Segment::Parameter(0)],
                vec![Segment::Text("--fixed".to_owned())],
                vec![
                    Segment::Parameter(0),
                    Segment::Text(":".to_owned()),
                    Segment::Parameter(0)
                ],
            ]
        );
    }

    #[test]
    fn rejects_invalid_argument_templates() {
        for arguments in [
            r#"["{other}"]"#,
            r#"["{host"]"#,
            r#"["host}"]"#,
            r#"["{}","{host}"]"#,
            r#"["{{host}}"]"#,
            r#"["{ho{host}st}"]"#,
            r#"["","{host}"]"#,
            r#"["{host}","a\nb"]"#,
            r#"["--fixed"]"#,
            "[]",
        ] {
            assert!(with("arguments", arguments).is_err(), "{arguments}");
        }
        assert!(error(with("arguments", r#"["--fixed"]"#)).contains("is not used in any argument"));
    }

    #[test]
    fn limits_list_sizes() {
        let arguments: Vec<String> = (0..33).map(|_| r#""{host}""#.to_owned()).collect();
        assert!(
            error(with("arguments", &format!("[{}]", arguments.join(","))))
                .contains("more than 32")
        );
        let parameters: Vec<String> = (0..9)
            .map(|index| format!(r#"{{"name":"p{index}","type":"host"}}"#))
            .collect();
        assert!(error(parameter(&parameters.join(","))).contains("more than 8"));
    }

    #[test]
    fn collects_several_errors() {
        let errors = protocol(
            r#"{"scheme":"http","name":"","program":"tool.exe","arguments":[],"parameters":[]}"#,
        )
        .unwrap_err();
        assert_eq!(errors.len(), 3, "{errors:?}");
    }
}
