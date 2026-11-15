use std::io::{self, Write};
use std::sync::Once;

use windows_sys::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole};

// State
static ATTACH: Once = Once::new();

// Output
pub fn print(text: &str) {
    attach();
    let _ = writeln!(io::stdout(), "{text}");
}

pub fn print_error(text: &str) {
    attach();
    let _ = writeln!(io::stderr(), "{text}");
}

// Console
fn attach() {
    ATTACH.call_once(|| unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    });
}
