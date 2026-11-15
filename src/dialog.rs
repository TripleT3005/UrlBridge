use std::ptr;

use windows_sys::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TD_ERROR_ICON, TD_WARNING_ICON,
    TDCBF_CANCEL_BUTTON, TDCBF_CLOSE_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TDF_SIZE_TO_CONTENT,
    TaskDialogIndirect,
};
use windows_sys::Win32::UI::WindowsAndMessaging::IDCANCEL;
use windows_sys::core::PCWSTR;

use crate::plan::Plan;
use crate::wide::to_wide;

// Constants
const TITLE: &str = "UrlBridge";
const START_BUTTON: i32 = 100;
const START_LABEL: &str = "Start";

// Dialogs
pub fn confirm(plan: &Plan) -> bool {
    let instruction = format!("Start {}?", plan.protocol.name);
    let arguments = plan.argument_line();
    let content = format!(
        "A link wants to start this program. Continue only if you opened the link yourself.\n\nProgram:\n{}\n\nArguments:\n{}",
        plan.protocol.program.path,
        if arguments.is_empty() {
            "(none)"
        } else {
            &arguments
        }
    );
    let label = to_wide(START_LABEL);
    let buttons = [TASKDIALOG_BUTTON {
        nButtonID: START_BUTTON,
        pszButtonText: label.as_ptr(),
    }];
    show(
        &instruction,
        &content,
        TD_WARNING_ICON,
        TDCBF_CANCEL_BUTTON,
        &buttons,
    ) == Some(START_BUTTON)
}

pub fn error(instruction: &str, content: &str) {
    show(instruction, content, TD_ERROR_ICON, TDCBF_CLOSE_BUTTON, &[]);
}

fn show(
    instruction: &str,
    content: &str,
    icon: PCWSTR,
    common_buttons: i32,
    buttons: &[TASKDIALOG_BUTTON],
) -> Option<i32> {
    let title = to_wide(TITLE);
    let instruction = to_wide(instruction);
    let content = to_wide(content);
    let config = TASKDIALOGCONFIG {
        cbSize: u32::try_from(size_of::<TASKDIALOGCONFIG>()).unwrap_or(0),
        dwFlags: TDF_ALLOW_DIALOG_CANCELLATION | TDF_SIZE_TO_CONTENT,
        dwCommonButtons: common_buttons,
        pszWindowTitle: title.as_ptr(),
        Anonymous1: TASKDIALOGCONFIG_0 { pszMainIcon: icon },
        pszMainInstruction: instruction.as_ptr(),
        pszContent: content.as_ptr(),
        cButtons: u32::try_from(buttons.len()).unwrap_or(0),
        pButtons: if buttons.is_empty() {
            ptr::null()
        } else {
            buttons.as_ptr()
        },
        nDefaultButton: if buttons.is_empty() { 0 } else { IDCANCEL },
        ..Default::default()
    };
    let mut pressed = 0;
    let result = unsafe {
        TaskDialogIndirect(
            &raw const config,
            &raw mut pressed,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    (result >= 0).then_some(pressed)
}
