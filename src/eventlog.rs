use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows_sys::Win32::System::EventLog::{
    DeregisterEventSource, EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE, EVENTLOG_WARNING_TYPE,
    RegisterEventSourceW, ReportEventW,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use crate::wide::to_wide;

// Constants
const SOURCE: &str = "UrlBridge";
const MAX_TEXT_LENGTH: usize = 512;
const TOKEN_USER_BUFFER: usize = 256;

// Events
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Started = 1,
    Cancelled = 2,
    Rejected = 3,
    InvalidConfiguration = 4,
    LaunchFailed = 5,
}

impl Event {
    fn kind(self) -> u16 {
        match self {
            Self::Started | Self::Cancelled => EVENTLOG_INFORMATION_TYPE,
            Self::Rejected => EVENTLOG_WARNING_TYPE,
            Self::InvalidConfiguration | Self::LaunchFailed => EVENTLOG_ERROR_TYPE,
        }
    }
}

pub fn write(event: Event, message: &str) {
    let source = to_wide(SOURCE);
    let log = unsafe { RegisterEventSourceW(ptr::null(), source.as_ptr()) };
    if log.is_null() {
        return;
    }
    let text = to_wide(message);
    let strings = [text.as_ptr()];
    let mut buffer = [0_u64; TOKEN_USER_BUFFER / 8];
    let user = current_user(&mut buffer);
    unsafe {
        ReportEventW(
            log,
            event.kind(),
            0,
            event as u32,
            user,
            1,
            0,
            strings.as_ptr(),
            ptr::null(),
        );
        DeregisterEventSource(log);
    }
}

pub fn sanitize(text: &str) -> String {
    let mut clean: String = text
        .chars()
        .take(MAX_TEXT_LENGTH)
        .map(|character| {
            if character.is_ascii_graphic() || character == ' ' {
                character
            } else {
                '?'
            }
        })
        .collect();
    if text.chars().count() > MAX_TEXT_LENGTH {
        clean.push_str("...");
    }
    clean
}

// User
fn current_user(buffer: &mut [u64; TOKEN_USER_BUFFER / 8]) -> *mut core::ffi::c_void {
    let mut token: HANDLE = ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) } == 0 {
        return ptr::null_mut();
    }
    let mut length = 0;
    let read = unsafe {
        GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            u32::try_from(TOKEN_USER_BUFFER).unwrap_or(0),
            &raw mut length,
        )
    };
    unsafe {
        CloseHandle(token);
    }
    if read == 0 {
        return ptr::null_mut();
    }
    unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_text() {
        assert_eq!(sanitize("a://pc 1"), "a://pc 1");
        assert_eq!(sanitize("a\r\nb\tc\0d"), "a??b?c?d");
        assert_eq!(sanitize("ä\u{202e}"), "??");
    }

    #[test]
    fn truncates_long_text() {
        let text = sanitize(&"a".repeat(600));
        assert_eq!(text.len(), MAX_TEXT_LENGTH + 3);
        assert!(text.ends_with("..."));
    }

    #[test]
    fn reads_the_current_user() {
        let mut buffer = [0; TOKEN_USER_BUFFER / 8];
        assert!(!current_user(&mut buffer).is_null());
    }
}
