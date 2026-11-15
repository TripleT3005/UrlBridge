use std::process::ExitCode;

// Exit codes
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    Success = 0,
    Usage = 1,
    InvalidConfiguration = 2,
    RejectedUrl = 3,
    Cancelled = 4,
    LaunchFailed = 5,
}

impl From<Exit> for ExitCode {
    fn from(exit: Exit) -> Self {
        ExitCode::from(exit as u8)
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_match_the_concept() {
        let codes = [
            Exit::Success,
            Exit::Usage,
            Exit::InvalidConfiguration,
            Exit::RejectedUrl,
            Exit::Cancelled,
            Exit::LaunchFailed,
        ]
        .map(|exit| exit as u8);
        assert_eq!(codes, [0, 1, 2, 3, 4, 5]);
    }
}
