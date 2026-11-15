use std::path::Path;
use std::ptr;

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError};
use windows_sys::Win32::System::Threading::{CreateProcessW, PROCESS_INFORMATION, STARTUPINFOW};

use crate::plan::Plan;
use crate::wide::to_wide;

// Launch
pub fn start(plan: &Plan) -> Result<(), u32> {
    let program = &plan.protocol.program.path;
    let application = to_wide(program);
    let mut command_line = to_wide(&plan.command_line());
    let directory = Path::new(program)
        .parent()
        .and_then(Path::to_str)
        .map(to_wide);
    let startup = STARTUPINFOW {
        cb: u32::try_from(size_of::<STARTUPINFOW>()).unwrap_or(0),
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();
    let created = unsafe {
        CreateProcessW(
            application.as_ptr(),
            command_line.as_mut_ptr(),
            ptr::null(),
            ptr::null(),
            0,
            0,
            ptr::null(),
            directory.as_ref().map_or(ptr::null(), Vec::as_ptr),
            &raw const startup,
            &raw mut process,
        )
    };
    if created == 0 {
        return Err(unsafe { GetLastError() });
    }
    unsafe {
        CloseHandle(process.hThread);
        CloseHandle(process.hProcess);
    }
    Ok(())
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;

    fn start_program(program: &str) -> Result<(), u32> {
        let json = format!(
            r#"{{"protocols":[{{"scheme":"abc","name":"A","program":"{}","arguments":["{{value}}"],"parameters":[{{"name":"value","type":"token","maxLength":8}}]}}]}}"#,
            program.replace('\\', r"\\")
        );
        let config = config::parse(json.as_bytes()).unwrap();
        let plan = Plan::new(&config, "abc://x").unwrap();
        start(&plan)
    }

    #[test]
    fn starts_a_program() {
        assert_eq!(start_program(r"C:\Windows\System32\hostname.exe"), Ok(()));
    }

    #[test]
    fn reports_a_missing_program() {
        const ERROR_FILE_NOT_FOUND: u32 = 2;
        assert_eq!(
            start_program(r"C:\Windows\System32\urlbridge-missing.exe"),
            Err(ERROR_FILE_NOT_FOUND)
        );
    }
}
