//! Console tools write redirected output in the OEM code page, not UTF-8.

use windows::Win32::Globalization::{GetOEMCP, MULTI_BYTE_TO_WIDE_CHAR_FLAGS, MultiByteToWideChar};

/// Decodes output of a console tool such as schtasks (localized messages included).
pub fn decode_oem(bytes: &[u8]) -> String {
    decode(bytes, unsafe { GetOEMCP() })
}

/// Decodes console output in the given code page.
pub fn decode(bytes: &[u8], code_page: u32) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    let flags = MULTI_BYTE_TO_WIDE_CHAR_FLAGS(0);
    // SAFETY: both buffers are valid slices; the first call only measures.
    let len = unsafe { MultiByteToWideChar(code_page, flags, bytes, None) };
    if len <= 0 {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let mut wide = vec![0u16; len as usize];
    let written = unsafe { MultiByteToWideChar(code_page, flags, bytes, Some(&mut wide)) };
    String::from_utf16_lossy(&wide[..written.max(0) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polish_oem_output_is_readable() {
        // "Błąd" in CP852, as schtasks prints it on a Polish system.
        assert_eq!(decode(b"B\x88\xa5d", 852), "Błąd");
    }

    #[test]
    fn ascii_is_unchanged() {
        assert_eq!(decode(b"ERROR: Access is denied.", 437), "ERROR: Access is denied.");
        assert_eq!(decode(b"", 852), "");
    }
}
