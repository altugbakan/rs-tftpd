use std::error::Error;

/// Allows conversions between byte arrays and other types.
///
/// # Example
///
/// ```rust
/// use tftpd::Convert;
///
/// assert_eq!(Convert::to_u16(&[0x01, 0x02]).unwrap(), 0x0102);
///
/// let (result, index) = Convert::to_string(b"hello world\0", 0).unwrap();
/// assert_eq!(result, "hello world");
/// assert_eq!(index, 11);
/// ```
pub struct Convert;

impl Convert {
    /// Converts a [`u8`] slice to a [`u16`].
    pub fn to_u16(buf: &[u8]) -> Result<u16, &'static str> {
        if buf.len() < 2 {
            Err("Error when converting to u16")
        } else {
            Ok(((buf[0] as u16) << 8) + buf[1] as u16)
        }
    }

    /// Converts a zero-terminated [`u8`] slice to a [`String`], and returns the
    /// size of the [`String`]. Useful for TFTP packet conversions.
    pub fn to_string(buf: &[u8], start: usize) -> Result<(String, usize), Box<dyn Error>> {
        match buf[start..].iter().position(|&b| b == 0x00) {
            Some(index) => Ok((
                String::from_utf8(buf[start..start + index].to_vec())?,
                index + start,
            )),
            None => Err("Invalid string".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_to_u16() {
        assert_eq!(Convert::to_u16(&[0x01, 0x02]).unwrap(), 0x0102);
        assert_eq!(Convert::to_u16(&[0x00, 0x02]).unwrap(), 0x0002);
        assert_eq!(Convert::to_u16(&[0xfe, 0xdc, 0xba]).unwrap(), 0xfedc);
    }

    #[test]
    fn returns_error_on_short_array() {
        assert!(Convert::to_u16(&[0x01]).is_err());
        assert!(Convert::to_u16(&[]).is_err());
    }

    #[test]
    fn converts_to_string() {
        let (result, index) = Convert::to_string(b"hello world\0", 0).unwrap();
        assert_eq!(result, "hello world");
        assert_eq!(index, 11);

        let (result, index) = Convert::to_string(b"hello\0world", 0).unwrap();
        assert_eq!(result, "hello");
        assert_eq!(index, 5);

        let (result, index) = Convert::to_string(b"\0hello world", 0).unwrap();
        assert_eq!(result, "");
        assert_eq!(index, 0);
    }

    #[test]
    fn converts_to_string_with_index() {
        let (result, index) = Convert::to_string(b"hello\0world\0", 0).unwrap();
        assert_eq!(result, "hello");
        assert_eq!(index, 5);

        let (result, index) = Convert::to_string(b"hello\0world\0", 5).unwrap();
        assert_eq!(result, "");
        assert_eq!(index, 5);

        let (result, index) = Convert::to_string(b"hello\0world\0", 6).unwrap();
        assert_eq!(result, "world");
        assert_eq!(index, 11);
    }
}
