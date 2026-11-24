use std::ptr;

use windows_sys::Win32::UI::Controls::{
    TASKDIALOG_BUTTON, TASKDIALOGCONFIG, TASKDIALOGCONFIG_0, TD_ERROR_ICON, TD_WARNING_ICON,
    TDF_ALLOW_DIALOG_CANCELLATION, TDF_RTL_LAYOUT, TDF_SIZE_TO_CONTENT, TaskDialogIndirect,
};
use windows_sys::Win32::UI::WindowsAndMessaging::IDCANCEL;
use windows_sys::core::PCWSTR;

use crate::i18n::{self, Translations};
use crate::plan::Plan;
use crate::wide::to_wide;

// Constants
const TITLE: &str = "UrlBridge";
const START_BUTTON: i32 = 100;
const LEFT_TO_RIGHT_MARK: char = '\u{200E}';

// Dialogs
pub fn confirm(plan: &Plan, texts: &Translations) -> bool {
    let instruction = texts.format(i18n::CONFIRM_TITLE, &[("name", &plan.protocol.name)]);
    let arguments = plan.argument_line();
    let content = format!(
        "{}\n\n{}\n{}\n\n{}\n{}",
        texts.text(i18n::CONFIRM_TEXT),
        texts.text(i18n::CONFIRM_PROGRAM),
        left_to_right(&plan.protocol.program.path),
        texts.text(i18n::CONFIRM_ARGUMENTS),
        if arguments.is_empty() {
            texts.text(i18n::CONFIRM_NO_ARGUMENTS)
        } else {
            left_to_right(&arguments)
        }
    );
    let buttons = [
        (START_BUTTON, texts.text(i18n::CONFIRM_START)),
        (IDCANCEL, texts.text(i18n::CONFIRM_CANCEL)),
    ];
    show(&instruction, &content, TD_WARNING_ICON, &buttons, texts) == Some(START_BUTTON)
}

pub fn error(instruction: &str, content: &str, texts: &Translations) {
    let buttons = [(IDCANCEL, texts.text(i18n::MESSAGE_CLOSE))];
    show(instruction, content, TD_ERROR_ICON, &buttons, texts);
}

fn show(
    instruction: &str,
    content: &str,
    icon: PCWSTR,
    buttons: &[(i32, String)],
    texts: &Translations,
) -> Option<i32> {
    let title = to_wide(TITLE);
    let instruction = to_wide(instruction);
    let content = to_wide(content);
    let labels: Vec<Vec<u16>> = buttons.iter().map(|(_, label)| to_wide(label)).collect();
    let buttons: Vec<TASKDIALOG_BUTTON> = buttons
        .iter()
        .zip(&labels)
        .map(|((id, _), label)| TASKDIALOG_BUTTON {
            nButtonID: *id,
            pszButtonText: label.as_ptr(),
        })
        .collect();
    let mut flags = TDF_ALLOW_DIALOG_CANCELLATION | TDF_SIZE_TO_CONTENT;
    if texts.is_right_to_left() {
        flags |= TDF_RTL_LAYOUT;
    }
    let config = TASKDIALOGCONFIG {
        cbSize: u32::try_from(size_of::<TASKDIALOGCONFIG>()).unwrap_or(0),
        dwFlags: flags,
        pszWindowTitle: title.as_ptr(),
        Anonymous1: TASKDIALOGCONFIG_0 { pszMainIcon: icon },
        pszMainInstruction: instruction.as_ptr(),
        pszContent: content.as_ptr(),
        cButtons: u32::try_from(buttons.len()).unwrap_or(0),
        pButtons: buttons.as_ptr(),
        nDefaultButton: IDCANCEL,
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

// Text
fn left_to_right(text: &str) -> String {
    format!("{LEFT_TO_RIGHT_MARK}{text}{LEFT_TO_RIGHT_MARK}")
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_commands_left_to_right() {
        let marked = left_to_right("/v:pc0042 /prompt");
        assert_eq!(marked, "\u{200E}/v:pc0042 /prompt\u{200E}");
        assert_eq!(marked.trim_matches(LEFT_TO_RIGHT_MARK), "/v:pc0042 /prompt");
    }
}
