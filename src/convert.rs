use std::error::Error;

pub struct Convert;

impl Convert {
    pub fn to_u16(buf: &[u8]) -> Result<u16, &'static str> {
        if buf.len() < 2 {
            Err("error when converting to u16")
        } else {
            Ok(((buf[0] as u16) << 8) + buf[1] as u16)
        }
    }

    pub fn to_string(buf: &[u8]) -> Result<(String, usize), Box<dyn Error>> {
        let zero_index = match buf[2..].iter().position(|&b| b == 0x00) {
            Some(index) => index,
            None => return Err("invalid string".into()),
        };

        Ok((String::from_utf8(buf[2..zero_index].to_vec())?, zero_index))
    }
}
