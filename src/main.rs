#![cfg_attr(not(test), windows_subsystem = "windows")]

mod app;
#[cfg(test)]
mod changelog;
mod cli;
mod config;
mod console;
mod dialog;
mod eventlog;
mod exit;
#[cfg(test)]
mod fuzz;
mod json;
mod launch;
mod plan;
mod security;
mod url;
mod value;
mod wide;

use std::env;
use std::process::ExitCode;

use windows_sys::Win32::System::LibraryLoader::{
    LOAD_LIBRARY_SEARCH_SYSTEM32, SetDefaultDllDirectories,
};

use cli::{Command, UsageError};
use exit::Exit;

// Entry point
fn main() -> ExitCode {
    unsafe {
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32);
    }
    let arguments: Option<Vec<String>> = env::args_os()
        .skip(1)
        .map(|argument| argument.into_string().ok())
        .collect();
    let Some(arguments) = arguments else {
        console::print_error("The command line contains invalid characters.");
        return Exit::Usage.into();
    };
    let exit = match cli::parse(&arguments) {
        Ok(command) => run(&command),
        Err(error) => {
            if error == UsageError::TooManyArguments {
                app::malformed_command_line(&arguments);
            }
            console::print_error(&error.to_string());
            console::print(cli::USAGE);
            Exit::Usage
        }
    };
    exit.into()
}

fn run(command: &Command) -> Exit {
    match command {
        Command::Help => {
            console::print(cli::USAGE);
            Exit::Success
        }
        Command::Version => {
            console::print(env!("CARGO_PKG_VERSION"));
            Exit::Success
        }
        Command::Open(url) => app::open(url),
        Command::Test(url) => app::test(url),
        Command::Validate => app::validate(),
    }
}
