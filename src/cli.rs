use crate::i18n::{self, Translations};

// Constants
const TEST: &str = "--test";
const VALIDATE: &str = "--validate";
const VERSION: &str = "--version";
const HELP: &str = "--help";
const OPTION_PREFIX: &str = "-";

// Command
#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Open(String),
    Test(String),
    Validate,
    Version,
    Help,
}

#[derive(Debug, PartialEq, Eq)]
pub enum UsageError {
    UnknownOption(String),
    MissingUrl,
    TooManyArguments,
}

impl UsageError {
    pub fn message(&self, texts: &Translations) -> String {
        match self {
            Self::UnknownOption(option) => {
                texts.format(i18n::CONSOLE_USAGE_UNKNOWN_OPTION, &[("option", option)])
            }
            Self::MissingUrl => texts.text(i18n::CONSOLE_USAGE_MISSING_LINK),
            Self::TooManyArguments => texts.text(i18n::CONSOLE_USAGE_TOO_MANY_ARGUMENTS),
        }
    }
}

// Parsing
pub fn parse(arguments: &[String]) -> Result<Command, UsageError> {
    let mut arguments = arguments.iter().map(String::as_str);
    let command = match arguments.next() {
        None | Some(HELP) => Command::Help,
        Some(VERSION) => Command::Version,
        Some(VALIDATE) => Command::Validate,
        Some(TEST) => Command::Test(arguments.next().ok_or(UsageError::MissingUrl)?.to_owned()),
        Some(option) if option.starts_with(OPTION_PREFIX) => {
            return Err(UsageError::UnknownOption(option.to_owned()));
        }
        Some(url) => Command::Open(url.to_owned()),
    };
    if arguments.next().is_some() {
        return Err(UsageError::TooManyArguments);
    }
    Ok(command)
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    fn parse_all(arguments: &[&str]) -> Result<Command, UsageError> {
        let arguments: Vec<String> = arguments.iter().map(|&text| text.to_owned()).collect();
        parse(&arguments)
    }

    #[test]
    fn shows_help_without_arguments() {
        assert_eq!(parse_all(&[]), Ok(Command::Help));
        assert_eq!(parse_all(&["--help"]), Ok(Command::Help));
    }

    #[test]
    fn opens_a_single_url() {
        assert_eq!(
            parse_all(&["bridge-rdp://pc0042"]),
            Ok(Command::Open("bridge-rdp://pc0042".to_owned()))
        );
    }

    #[test]
    fn rejects_additional_arguments_after_a_url() {
        assert_eq!(
            parse_all(&["bridge-rdp://pc0042", "/admin"]),
            Err(UsageError::TooManyArguments)
        );
    }

    #[test]
    fn requires_a_url_for_test() {
        assert_eq!(parse_all(&["--test"]), Err(UsageError::MissingUrl));
        assert_eq!(
            parse_all(&["--test", "bridge-rdp://pc0042"]),
            Ok(Command::Test("bridge-rdp://pc0042".to_owned()))
        );
    }

    #[test]
    fn rejects_unknown_options() {
        assert_eq!(
            parse_all(&["--register"]),
            Err(UsageError::UnknownOption("--register".to_owned()))
        );
        assert_eq!(
            parse_all(&["/v:host"]),
            Ok(Command::Open("/v:host".to_owned()))
        );
    }

    #[test]
    fn rejects_arguments_after_options() {
        assert_eq!(
            parse_all(&["--validate", "x"]),
            Err(UsageError::TooManyArguments)
        );
    }
}
