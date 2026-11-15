use std::env;
use std::fs;
use std::path::Path;

use crate::config::{self, Config, Location, Protocol};
use crate::console;
use crate::dialog;
use crate::eventlog::{self, Event};
use crate::exit::Exit;
use crate::i18n::{self, Translations};
use crate::json;
use crate::launch;
use crate::plan::Plan;
use crate::security::{self, Problem};

// Constants
const CONFIG_FILE: &str = "UrlBridge.json";
const INTERPRETERS: &[&str] = &[
    "bash.exe",
    "cmd.exe",
    "conhost.exe",
    "cscript.exe",
    "msiexec.exe",
    "mshta.exe",
    "powershell.exe",
    "powershell_ise.exe",
    "pwsh.exe",
    "regsvr32.exe",
    "rundll32.exe",
    "wscript.exe",
    "wsl.exe",
    "wt.exe",
];

// Loading
struct Loaded {
    config: Config,
    texts: Translations,
    warnings: Vec<String>,
}

fn load() -> Result<Loaded, Vec<String>> {
    let executable = env::current_exe()
        .ok()
        .and_then(|path| path.to_str().map(str::to_owned))
        .ok_or_else(|| vec!["The path of UrlBridge.exe cannot be determined.".to_owned()])?;
    let config_path = Path::new(&executable)
        .with_file_name(CONFIG_FILE)
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| vec!["The path of the configuration cannot be determined.".to_owned()])?;
    load_from(&executable, &config_path)
}

fn load_from(executable: &str, config_path: &str) -> Result<Loaded, Vec<String>> {
    let mut errors = Vec::new();
    for path in [executable, config_path] {
        if let Err(problem) = security::check_file(path) {
            errors.push(problem);
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let config = config::parse(&read(config_path).map_err(|problem| vec![problem])?)?;
    let texts = Translations::load(config.language.as_deref());
    let mut warnings = texts.warnings().to_vec();
    for protocol in &config.protocols {
        check_program(protocol, &texts, &mut errors, &mut warnings);
    }
    if errors.is_empty() {
        Ok(Loaded {
            config,
            texts,
            warnings,
        })
    } else {
        Err(errors)
    }
}

fn read(path: &str) -> Result<Vec<u8>, String> {
    let too_large = fs::metadata(path).is_ok_and(|metadata| {
        usize::try_from(metadata.len()).is_ok_and(|size| size > json::MAX_SIZE)
    });
    if too_large {
        return Err(format!("{path} is larger than 64 KiB."));
    }
    fs::read(path).map_err(|error| format!("{path} cannot be read: {error}."))
}

fn check_program(
    protocol: &Protocol,
    texts: &Translations,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    let path = &protocol.program.path;
    let scheme = &protocol.scheme;
    match check_program_file(protocol) {
        Ok(()) => {}
        Err(problem @ Problem::Missing(_)) => {
            warnings.push(format!("{scheme}: {}", problem_text(&problem, texts)));
        }
        Err(Problem::Unsafe(problem)) => errors.push(format!("{scheme}: {problem}")),
    }
    let is_interpreter = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            INTERPRETERS
                .iter()
                .any(|interpreter| name.eq_ignore_ascii_case(interpreter))
        });
    if is_interpreter {
        warnings.push(format!(
            "{scheme}: {path} is an interpreter. Values from a link can become code there."
        ));
    }
}

fn check_program_file(protocol: &Protocol) -> Result<(), Problem> {
    let path = &protocol.program.path;
    match protocol.program.location {
        Location::Local => security::check_local_program(path),
        Location::Network => security::check_network_program(path),
    }
}

fn problem_text(problem: &Problem, texts: &Translations) -> String {
    match problem {
        Problem::Missing(path) => texts.format(i18n::MESSAGE_LAUNCH_NOT_FOUND, &[("path", path)]),
        Problem::Unsafe(message) => message.clone(),
    }
}

// Commands
pub fn texts() -> Translations {
    load().map_or_else(|_| Translations::load(None), |loaded| loaded.texts)
}

pub fn usage(texts: &Translations) {
    console::print(&texts.text(i18n::CONSOLE_USAGE));
}

pub fn validate() -> Exit {
    match load() {
        Ok(loaded) => {
            let texts = &loaded.texts;
            print_warnings(&loaded.warnings, texts);
            console::print(&texts.format(
                i18n::CONSOLE_LANGUAGE,
                &[
                    ("name", &texts.text(i18n::LANGUAGE_NAME)),
                    ("code", texts.code()),
                ],
            ));
            let schemes: Vec<&str> = loaded
                .config
                .protocols
                .iter()
                .map(|protocol| protocol.scheme.as_str())
                .collect();
            let list = if schemes.is_empty() {
                texts.text(i18n::CONSOLE_NO_PROTOCOLS)
            } else {
                schemes.join(", ")
            };
            console::print(&texts.format(
                i18n::CONSOLE_VALID,
                &[("count", &schemes.len().to_string()), ("schemes", &list)],
            ));
            Exit::Success
        }
        Err(errors) => {
            let texts = Translations::load(None);
            print_errors(&errors, &texts);
            console::print_error(&texts.text(i18n::CONSOLE_INVALID));
            Exit::InvalidConfiguration
        }
    }
}

pub fn test(url: &str) -> Exit {
    let loaded = match load() {
        Ok(loaded) => loaded,
        Err(errors) => {
            print_errors(&errors, &Translations::load(None));
            return Exit::InvalidConfiguration;
        }
    };
    let texts = &loaded.texts;
    match Plan::new(&loaded.config, url) {
        Ok(plan) => {
            console::print(&texts.format(
                i18n::CONSOLE_TEST,
                &[
                    ("name", &plan.protocol.name),
                    ("scheme", &plan.protocol.scheme),
                    ("program", &plan.protocol.program.path),
                    ("arguments", &plan.argument_line()),
                    ("command_line", &plan.command_line()),
                ],
            ));
            if let Err(problem) = check_program_file(plan.protocol) {
                print_warnings(&[problem_text(&problem, texts)], texts);
            }
            console::print(&texts.text(i18n::CONSOLE_NOTHING_STARTED));
            Exit::Success
        }
        Err(rejection) => {
            console::print_error(&texts.format(
                i18n::CONSOLE_REJECTED,
                &[("text", &rejection.message(texts))],
            ));
            Exit::RejectedUrl
        }
    }
}

pub fn open(url: &str) -> Exit {
    let english = Translations::english();
    let shown_url = eventlog::sanitize(url);
    let loaded = match load() {
        Ok(loaded) => loaded,
        Err(errors) => {
            let texts = Translations::load(None);
            print_errors(&errors, &texts);
            eventlog::write(
                Event::InvalidConfiguration,
                &format!(
                    "Nothing was started because the configuration is invalid.\nURL: {shown_url}\n{}",
                    errors.join("\n")
                ),
            );
            dialog::error(
                &texts.text(i18n::MESSAGE_CONFIGURATION_TITLE),
                &texts.text(i18n::MESSAGE_CONFIGURATION_TEXT),
                &texts,
            );
            return Exit::InvalidConfiguration;
        }
    };
    let texts = &loaded.texts;
    let plan = match Plan::new(&loaded.config, url) {
        Ok(plan) => plan,
        Err(rejection) => {
            let reason = rejection.message(texts);
            console::print_error(&texts.format(i18n::CONSOLE_REJECTED, &[("text", &reason)]));
            eventlog::write(
                Event::Rejected,
                &format!(
                    "The link was rejected.\nURL: {shown_url}\nReason: {}",
                    rejection.message(&english)
                ),
            );
            dialog::error(
                &texts.text(i18n::MESSAGE_REJECTED_TITLE),
                &format!("{}\n\n{reason}", texts.text(i18n::MESSAGE_REJECTED_TEXT)),
                texts,
            );
            return Exit::RejectedUrl;
        }
    };
    let details = format!(
        "Protocol: {} ({})\nProgram: {}\nArguments: {}\nURL: {shown_url}",
        plan.protocol.name,
        plan.protocol.scheme,
        plan.protocol.program.path,
        plan.argument_line()
    );
    if let Err(problem) = check_program_file(plan.protocol) {
        return launch_failed(
            &details,
            &problem.english(),
            &problem_text(&problem, texts),
            texts,
        );
    }
    if !dialog::confirm(&plan, texts) {
        eventlog::write(
            Event::Cancelled,
            &format!("The start was cancelled by the user.\n{details}"),
        );
        return Exit::Cancelled;
    }
    match launch::start(&plan) {
        Ok(()) => {
            eventlog::write(
                Event::Started,
                &format!("The program was started.\n{details}"),
            );
            Exit::Success
        }
        Err(code) => {
            let code = [("code", code.to_string())];
            let values = [(code[0].0, code[0].1.as_str())];
            launch_failed(
                &details,
                &english.format(i18n::MESSAGE_LAUNCH_WINDOWS_ERROR, &values),
                &texts.format(i18n::MESSAGE_LAUNCH_WINDOWS_ERROR, &values),
                texts,
            )
        }
    }
}

pub fn malformed_command_line(arguments: &[String]) {
    eventlog::write(
        Event::Rejected,
        &format!(
            "The link was rejected.\nCommand line: {}\nReason: The command line contains more than one argument.",
            eventlog::sanitize(&arguments.join(" "))
        ),
    );
}

fn launch_failed(details: &str, logged: &str, shown: &str, texts: &Translations) -> Exit {
    console::print_error(&texts.format(i18n::CONSOLE_LAUNCH_FAILED, &[("text", shown)]));
    eventlog::write(
        Event::LaunchFailed,
        &format!("The program could not be started.\n{details}\nReason: {logged}"),
    );
    dialog::error(&texts.text(i18n::MESSAGE_LAUNCH_TITLE), shown, texts);
    Exit::LaunchFailed
}

fn print_errors(errors: &[String], texts: &Translations) {
    for error in errors {
        console::print_error(&texts.format(i18n::CONSOLE_ERROR, &[("text", error)]));
    }
}

fn print_warnings(warnings: &[String], texts: &Translations) {
    for warning in warnings {
        console::print(&texts.format(i18n::CONSOLE_WARNING, &[("text", warning)]));
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_a_configuration_that_users_can_modify() {
        let directory = env::temp_dir().join(format!("urlbridge-app-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let executable = directory.join("UrlBridge.exe");
        let config = directory.join(CONFIG_FILE);
        fs::write(&executable, b"test").unwrap();
        fs::write(&config, br#"{"protocols":[]}"#).unwrap();
        let result = load_from(executable.to_str().unwrap(), config.to_str().unwrap());
        fs::remove_dir_all(&directory).unwrap();
        assert_eq!(result.err().map(|errors| errors.len()), Some(2));
    }

    #[test]
    fn warns_about_interpreters_and_missing_programs() {
        let json = br#"{"protocols":[
            {"scheme":"abc","name":"A","program":"C:\\Windows\\System32\\cmd.exe","arguments":[],"parameters":[]},
            {"scheme":"abd","name":"B","program":"C:\\Windows\\System32\\urlbridge-missing.exe","arguments":[],"parameters":[]},
            {"scheme":"abe","name":"C","program":"C:\\Windows\\System32\\mstsc.exe","arguments":[],"parameters":[]}
        ]}"#;
        let config = config::parse(json).unwrap();
        let (mut errors, mut warnings) = (Vec::new(), Vec::new());
        for protocol in &config.protocols {
            check_program(
                protocol,
                &Translations::english(),
                &mut errors,
                &mut warnings,
            );
        }
        assert_eq!(errors, Vec::<String>::new());
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(warnings[0].contains("interpreter"));
        assert!(warnings[1].starts_with("abd:"));
        assert!(warnings[1].contains("was not found"));
    }
}
