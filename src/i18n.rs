use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::ptr;

use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};

use crate::security;

// Constants
const ENGLISH: &str = include_str!("../lang/en.lang");
const ENGLISH_CODE: &str = "en";
const FOLDER: &str = "lang";
const EXTENSION: &str = "lang";
const MAX_FILE_SIZE: u64 = 256 * 1024;
const MAX_CODE_LENGTH: usize = 35;
const RIGHT_TO_LEFT: &str = "rtl";

macro_rules! keys {
    ($($name:ident = $key:literal;)*) => {
        $(pub const $name: &str = $key;)*

        #[cfg(test)]
        const KEYS: &[&str] = &[$($name),*];
    };
}

keys! {
    LANGUAGE_NAME = "language.name";
    CONFIRM_TITLE = "confirm.title";
    CONFIRM_TEXT = "confirm.text";
    CONFIRM_PROGRAM = "confirm.program";
    CONFIRM_ARGUMENTS = "confirm.arguments";
    CONFIRM_NO_ARGUMENTS = "confirm.no_arguments";
    CONFIRM_START = "confirm.start";
    CONFIRM_CANCEL = "confirm.cancel";
    MESSAGE_CLOSE = "message.close";
    MESSAGE_CONFIGURATION_TITLE = "message.configuration.title";
    MESSAGE_CONFIGURATION_TEXT = "message.configuration.text";
    MESSAGE_REJECTED_TITLE = "message.rejected.title";
    MESSAGE_REJECTED_TEXT = "message.rejected.text";
    MESSAGE_LAUNCH_TITLE = "message.launch.title";
    MESSAGE_LAUNCH_NOT_FOUND = "message.launch.not_found";
    MESSAGE_LAUNCH_WINDOWS_ERROR = "message.launch.windows_error";
    REJECTION_TOO_LONG = "rejection.too_long";
    REJECTION_INVALID_CHARACTER = "rejection.invalid_character";
    REJECTION_MISSING_SCHEME = "rejection.missing_scheme";
    REJECTION_MISSING_SLASHES = "rejection.missing_slashes";
    REJECTION_QUERY = "rejection.query";
    REJECTION_FRAGMENT = "rejection.fragment";
    REJECTION_USER_INFO = "rejection.user_info";
    REJECTION_EMPTY_VALUE = "rejection.empty_value";
    REJECTION_INVALID_ENCODING = "rejection.invalid_encoding";
    REJECTION_UNKNOWN_PROTOCOL = "rejection.unknown_protocol";
    REJECTION_VALUE_COUNT = "rejection.value_count";
    REJECTION_VALUE_UNSAFE = "rejection.value.unsafe";
    REJECTION_VALUE_HOSTNAME = "rejection.value.hostname";
    REJECTION_VALUE_IPV4 = "rejection.value.ipv4";
    REJECTION_VALUE_HOST = "rejection.value.host";
    REJECTION_VALUE_INTEGER = "rejection.value.integer";
    REJECTION_VALUE_TOKEN = "rejection.value.token";
    REJECTION_VALUE_CHOICE = "rejection.value.choice";
    CONSOLE_USAGE = "console.usage";
    CONSOLE_USAGE_UNKNOWN_OPTION = "console.usage.unknown_option";
    CONSOLE_USAGE_MISSING_LINK = "console.usage.missing_link";
    CONSOLE_USAGE_TOO_MANY_ARGUMENTS = "console.usage.too_many_arguments";
    CONSOLE_USAGE_INVALID_CHARACTERS = "console.usage.invalid_characters";
    CONSOLE_ERROR = "console.error";
    CONSOLE_WARNING = "console.warning";
    CONSOLE_REJECTED = "console.rejected";
    CONSOLE_LANGUAGE = "console.language";
    CONSOLE_VALID = "console.valid";
    CONSOLE_NO_PROTOCOLS = "console.no_protocols";
    CONSOLE_INVALID = "console.invalid";
    CONSOLE_TEST = "console.test";
    CONSOLE_NOTHING_STARTED = "console.nothing_started";
    CONSOLE_LAUNCH_FAILED = "console.launch_failed";
}

pub const LANGUAGE_FALLBACK: &str = "language.fallback";
pub const LANGUAGE_DIRECTION: &str = "language.direction";

#[cfg(test)]
const OPTIONAL_KEYS: [&str; 2] = [LANGUAGE_FALLBACK, LANGUAGE_DIRECTION];

// Translations
pub struct Translations {
    code: String,
    texts: HashMap<String, String>,
    warnings: Vec<String>,
}

impl Translations {
    pub fn english() -> Self {
        Self {
            code: ENGLISH_CODE.to_owned(),
            texts: parse(ENGLISH),
            warnings: Vec::new(),
        }
    }

    pub fn load(preference: Option<&str>) -> Self {
        match directory() {
            Some(directory) => Self::load_from(&directory, preference, &preferred_ui_languages()),
            None => Self::english(),
        }
    }

    fn load_from(directory: &Path, preference: Option<&str>, preferred: &[String]) -> Self {
        let mut warnings = Vec::new();
        let codes = available_codes(directory);
        let code = resolve_code(preference, preferred, &codes);
        if let Some(wanted) = preference
            && !candidates(wanted)
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(&code))
        {
            warnings.push(format!(
                "The configured language \"{wanted}\" has no language file. Using \"{code}\"."
            ));
        }
        let texts = load_texts(&code, directory, &mut warnings);
        let usable = code.eq_ignore_ascii_case(ENGLISH_CODE)
            || !read_language(&code, directory, &mut Vec::new()).is_empty();
        Self {
            code: if usable {
                code
            } else {
                ENGLISH_CODE.to_owned()
            },
            texts,
            warnings,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn text(&self, key: &str) -> String {
        self.texts
            .get(key)
            .cloned()
            .unwrap_or_else(|| key.to_owned())
    }

    pub fn format(&self, key: &str, values: &[(&str, &str)]) -> String {
        fill(&self.text(key), values)
    }

    pub fn is_right_to_left(&self) -> bool {
        self.texts
            .get(LANGUAGE_DIRECTION)
            .is_some_and(|direction| direction.eq_ignore_ascii_case(RIGHT_TO_LEFT))
    }
}

fn fill(text: &str, values: &[(&str, &str)]) -> String {
    let mut result = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        result.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let value = after.find('}').and_then(|end| {
            values
                .iter()
                .find(|(name, _)| *name == &after[..end])
                .map(|(_, value)| (end, *value))
        });
        if let Some((end, value)) = value {
            result.push_str(value);
            rest = &after[end + 1..];
        } else {
            result.push('{');
            rest = after;
        }
    }
    result.push_str(rest);
    result
}

// Loading
fn directory() -> Option<PathBuf> {
    Some(std::env::current_exe().ok()?.parent()?.join(FOLDER))
}

fn load_texts(code: &str, directory: &Path, warnings: &mut Vec<String>) -> HashMap<String, String> {
    let fallback = read_language(code, directory, &mut Vec::new())
        .remove(LANGUAGE_FALLBACK)
        .filter(|fallback| is_valid_code(fallback));
    let mut texts = parse(ENGLISH);
    for code in language_chain(code, fallback.as_deref()) {
        texts.extend(read_language(code, directory, warnings));
    }
    texts
}

fn read_language(
    code: &str,
    directory: &Path,
    warnings: &mut Vec<String>,
) -> HashMap<String, String> {
    let path = directory.join(format!("{code}.{EXTENSION}"));
    let Some(name) = path.to_str() else {
        return HashMap::new();
    };
    if !path.is_file() {
        return HashMap::new();
    }
    if let Err(problem) = security::check_file(name) {
        warnings.push(format!("The language file {name} is ignored: {problem}"));
        return HashMap::new();
    }
    read_file(&path).map_or_else(HashMap::new, |content| parse(&content))
}

fn language_chain<'a>(code: &'a str, fallback: Option<&'a str>) -> Vec<&'a str> {
    let mut chain = Vec::new();
    let candidates = fallback
        .map(fallback_chain)
        .unwrap_or_default()
        .into_iter()
        .chain(fallback_chain(code));
    for candidate in candidates {
        if !chain
            .iter()
            .any(|known: &&str| known.eq_ignore_ascii_case(candidate))
        {
            chain.push(candidate);
        }
    }
    chain
}

// Languages
fn available_codes(directory: &Path) -> Vec<String> {
    let mut codes = vec![ENGLISH_CODE.to_owned()];
    let Ok(files) = fs::read_dir(directory) else {
        return codes;
    };
    for file in files.flatten() {
        let path = file.path();
        if !path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case(EXTENSION))
        {
            continue;
        }
        let Some(code) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if is_valid_code(code) && !codes.iter().any(|known| known.eq_ignore_ascii_case(code)) {
            codes.push(code.to_owned());
        }
    }
    codes
}

fn resolve_code(preference: Option<&str>, preferred: &[String], available: &[String]) -> String {
    let find = |code: &str| {
        available
            .iter()
            .find(|candidate| candidate.eq_ignore_ascii_case(code))
            .cloned()
    };
    preference
        .into_iter()
        .chain(preferred.iter().map(String::as_str))
        .find_map(|code| candidates(code).into_iter().find_map(find))
        .unwrap_or_else(|| ENGLISH_CODE.to_owned())
}

fn candidates(code: &str) -> Vec<&str> {
    let mut candidates: Vec<&str> = code
        .char_indices()
        .filter(|&(_, character)| character == '-')
        .map(|(index, _)| &code[..index])
        .rev()
        .collect();
    candidates.insert(0, code);
    candidates
}

fn fallback_chain(code: &str) -> Vec<&str> {
    let mut chain = vec![ENGLISH_CODE];
    for candidate in [base_code(code), code] {
        if !chain
            .iter()
            .any(|known| known.eq_ignore_ascii_case(candidate))
        {
            chain.push(candidate);
        }
    }
    chain
}

fn base_code(code: &str) -> &str {
    code.split('-').next().unwrap_or(code)
}

pub fn is_valid_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= MAX_CODE_LENGTH
        && code
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

fn preferred_ui_languages() -> Vec<String> {
    let mut count = 0_u32;
    let mut length = 0_u32;
    if unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &raw mut count,
            ptr::null_mut(),
            &raw mut length,
        )
    } == 0
    {
        return Vec::new();
    }
    let mut buffer = vec![0_u16; length as usize];
    if unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &raw mut count,
            buffer.as_mut_ptr(),
            &raw mut length,
        )
    } == 0
    {
        return Vec::new();
    }
    buffer
        .split(|&unit| unit == 0)
        .filter(|part| !part.is_empty())
        .map(String::from_utf16_lossy)
        .collect()
}

// Files
fn read_file(path: &Path) -> Option<String> {
    let metadata = fs::metadata(path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_SIZE {
        return None;
    }
    fs::read_to_string(path).ok()
}

pub fn parse(content: &str) -> HashMap<String, String> {
    content
        .trim_start_matches('\u{feff}')
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with(';'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim(), value.trim()))
        .filter(|(key, value)| !key.is_empty() && !value.is_empty())
        .map(|(key, value)| (key.to_owned(), value.replace("\\n", "\n")))
        .collect()
}

// Tests
#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn codes(values: &[&str]) -> Vec<String> {
        values.iter().map(|&value| value.to_owned()).collect()
    }

    fn placeholders(text: &str) -> BTreeSet<String> {
        text.split('{')
            .skip(1)
            .filter_map(|part| part.split_once('}'))
            .map(|(name, _)| name.to_owned())
            .collect()
    }

    fn bundled() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join(FOLDER)
    }

    #[test]
    fn parses_language_files() {
        let texts = parse(
            "\u{feff}# Comment\n; Comment\nconfirm.start = Start \nbroken\nempty =\nmulti = One\\nTwo\n",
        );
        assert_eq!(texts.len(), 2);
        assert_eq!(texts["confirm.start"], "Start");
        assert_eq!(texts["multi"], "One\nTwo");
    }

    #[test]
    fn resolves_the_language() {
        let available = codes(&["en", "de", "pt-BR"]);
        assert_eq!(
            resolve_code(None, &codes(&["de-AT", "en-US"]), &available),
            "de"
        );
        assert_eq!(resolve_code(None, &codes(&["pt-br"]), &available), "pt-BR");
        assert_eq!(resolve_code(None, &codes(&["fr-FR"]), &available), "en");
        assert_eq!(
            resolve_code(Some("en"), &codes(&["de-DE"]), &available),
            "en"
        );
        assert_eq!(
            resolve_code(Some("xx"), &codes(&["de-DE"]), &available),
            "de"
        );
        assert_eq!(candidates("sr-Latn-RS"), ["sr-Latn-RS", "sr-Latn", "sr"]);
    }

    #[test]
    fn falls_back_through_the_declared_language_and_english() {
        assert_eq!(language_chain("sxu", Some("de")), ["en", "de", "sxu"]);
        assert_eq!(language_chain("de-CH", None), ["en", "de", "de-CH"]);
        assert_eq!(fallback_chain("en"), ["en"]);
    }

    #[test]
    fn formats_placeholders_in_one_pass() {
        assert_eq!(
            fill("Hello {name}, {name}!", &[("name", "Ada")]),
            "Hello Ada, Ada!"
        );
        assert_eq!(
            fill("{a} and {b}", &[("a", "{b}"), ("b", "x")]),
            "{b} and x"
        );
        assert_eq!(fill("{unknown} {", &[("a", "x")]), "{unknown} {");
        assert_eq!(Translations::english().text("missing"), "missing");
    }

    #[test]
    fn checks_language_codes() {
        for code in ["en", "de-AT", "sr-Latn-RS", "de-x-amtsdt"] {
            assert!(is_valid_code(code), "{code}");
        }
        for code in ["", "de_AT", "../de", "de.lang", "a b", &"a".repeat(36)] {
            assert!(!is_valid_code(code), "{code}");
        }
    }

    #[test]
    fn ignores_language_files_that_users_can_modify() {
        let directory = std::env::temp_dir().join(format!("urlbridge-lang-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("xx.lang"), "confirm.start = Go\n").unwrap();
        let translations = Translations::load_from(&directory, Some("xx"), &[]);
        fs::remove_dir_all(&directory).unwrap();
        assert_eq!(translations.code(), "en");
        assert_eq!(translations.text(CONFIRM_START), "Start");
        assert_eq!(translations.warnings().len(), 1);
    }

    #[test]
    fn warns_about_a_configured_language_without_a_file() {
        let translations = Translations::load_from(&bundled(), Some("fr"), &codes(&["de-DE"]));
        assert!(translations.warnings()[0].contains("\"fr\""));
        assert!(translations.warnings()[0].contains("\"de\""));
        let regional = Translations::load_from(&bundled(), Some("de-AT"), &[]);
        assert!(
            regional
                .warnings()
                .iter()
                .all(|warning| !warning.contains("has no language file"))
        );
    }

    #[test]
    fn english_contains_every_text() {
        let english = parse(ENGLISH);
        for key in KEYS {
            assert!(english.contains_key(*key), "en.lang is missing {key}");
        }
        assert_eq!(
            english.len(),
            KEYS.len(),
            "en.lang contains texts that are not used"
        );
    }

    #[test]
    fn bundled_languages_match_english() {
        let english = parse(ENGLISH);
        for file in fs::read_dir(bundled()).unwrap().flatten() {
            let path = file.path();
            let texts = parse(&fs::read_to_string(&path).unwrap());
            let name = path.display();
            assert!(
                is_valid_code(path.file_stem().unwrap().to_str().unwrap()),
                "{name} has an invalid name"
            );
            for (key, text) in &texts {
                if OPTIONAL_KEYS.contains(&key.as_str()) {
                    continue;
                }
                let Some(original) = english.get(key) else {
                    panic!("{name} contains the unknown text {key}");
                };
                assert_eq!(
                    placeholders(text),
                    placeholders(original),
                    "{name}: {key} has other placeholders"
                );
            }
            for key in english.keys() {
                assert!(texts.contains_key(key), "{name} is missing {key}");
            }
        }
    }
}
