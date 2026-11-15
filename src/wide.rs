// Conversion
pub fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

pub fn from_wide_pointer(text: *const u16) -> String {
    if text.is_null() {
        return String::new();
    }
    let mut length = 0;
    unsafe {
        while *text.add(length) != 0 {
            length += 1;
        }
        String::from_utf16_lossy(std::slice::from_raw_parts(text, length))
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_text() {
        let wide = to_wide("Grüße");
        assert_eq!(wide.last(), Some(&0));
        assert_eq!(from_wide_pointer(wide.as_ptr()), "Grüße");
        assert_eq!(from_wide_pointer(std::ptr::null()), "");
    }
}
