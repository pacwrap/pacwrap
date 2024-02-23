use std::fmt::{Debug, Display};

use crate::{err, impl_error, Error, ErrorTrait, Result};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum BufferMode {
    Write,
    Read,
}

pub struct ByteBuffer {
    mode: Option<BufferMode>,
    position: usize,
    buffer: Vec<u8>,
}

enum BufferError {
    WriteOnRead,
    ReadOnWrite,
    ModeUnset,
    BufferOverrun(usize, usize),
}

impl_error!(BufferError);

impl Display for BufferError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{self:?}")
    }
}

impl Debug for BufferError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WriteOnRead => write!(fmt, "\"Write on read error\""),
            Self::ReadOnWrite => write!(fmt, "\"Read on write error\""),
            Self::ModeUnset => write!(fmt, "\"Buffer mode is unset\""),
            Self::BufferOverrun(req, aval) => write!(fmt, "\"Buffer overflow: {req} > {aval}\""),
        }
    }
}

impl From<Vec<u8>> for ByteBuffer {
    fn from(value: Vec<u8>) -> Self {
        Self {
            mode: None,
            position: 0,
            buffer: value,
        }
    }
}

impl ByteBuffer {
    pub fn new() -> Self {
        Self {
            mode: None,
            position: 0,
            buffer: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            mode: None,
            position: 0,
            buffer: vec![0; capacity],
        }
    }

    pub fn read(mut self) -> Self {
        self.mode = Some(BufferMode::Read);
        self.position = 0;
        self
    }

    pub fn write(mut self) -> Self {
        self.mode = Some(BufferMode::Write);
        self.position = 0;
        self
    }

    pub fn write_le_64(&mut self, value: u64) {
        self.insert_slice(&value.to_le_bytes())
    }

    pub fn read_le_64(&mut self) -> u64 {
        let read_position = self.reserve_position(8);

        self.check_read().unwrap();
        u64::from_le_bytes(self.buffer[read_position .. self.position].try_into().unwrap())
    }

    pub fn read_le_32(&mut self) -> u32 {
        let read_position = self.reserve_position(4);

        self.check_read().unwrap();
        u32::from_le_bytes(self.buffer[read_position .. self.position].try_into().unwrap())
    }

    pub fn write_le_32(&mut self, value: u32) {
        self.insert_slice(&value.to_le_bytes())
    }

    pub fn read_le_16(&mut self) -> u16 {
        let read_position = self.reserve_position(2);

        self.check_read().unwrap();
        u16::from_le_bytes(self.buffer[read_position .. self.position].try_into().unwrap())
    }

    pub fn write_le_16(&mut self, value: u16) {
        self.insert_slice(&value.to_le_bytes())
    }

    pub fn write_byte(&mut self, value: u8) {
        self.check_write().unwrap();
        self.reserve_position(1);
        self.buffer.push(value as u8)
    }

    pub fn read_byte(&mut self) -> u8 {
        let read_position = self.reserve_position(1);

        self.check_read().unwrap();
        self.buffer[read_position]
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    pub fn insert_slice(&mut self, array: &[u8]) {
        self.reserve_position(array.len());
        self.buffer.extend_from_slice(array);
    }

    fn reserve_position(&mut self, val: usize) -> usize {
        self.position += val;
        self.position - val
    }

    fn check_read(&self) -> Result<()> {
        if self.position > self.buffer.len() {
            err!(BufferError::BufferOverrun(self.position, self.buffer.len()))?
        }

        match self.mode {
            Some(mode) => match mode {
                BufferMode::Write => err!(BufferError::WriteOnRead),
                BufferMode::Read => Ok(()),
            },
            None => err!(BufferError::ModeUnset),
        }
    }

    fn check_write(&self) -> Result<()> {
        match self.mode {
            Some(mode) => match mode {
                BufferMode::Read => err!(BufferError::ReadOnWrite),
                BufferMode::Write => Ok(()),
            },
            None => err!(BufferError::ModeUnset),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::utils::bytebuffer::*;

    #[test]
    fn le_64() {
        let val1 = 7432597273248;
        let val2 = 4326434353598;
        let mut buffer = ByteBuffer::new().write();
        buffer.write_le_64(val1);
        buffer.write_le_64(val2);

        let mut buffer = buffer.read();

        assert_eq!(buffer.read_le_64(), val1);
        assert_eq!(buffer.read_le_64(), val2);
    }

    #[test]
    fn le_32() {
        let val1 = 743259727;
        let val2 = 432643435;
        let mut buffer = ByteBuffer::new().write();
        buffer.write_le_32(val1);
        buffer.write_le_32(val2);

        let mut buffer = buffer.read();

        assert_eq!(buffer.read_le_32(), val1);
        assert_eq!(buffer.read_le_32(), val2);
    }

    #[test]
    fn le_16() {
        let val1 = 23589;
        let val2 = 63236;
        let mut buffer = ByteBuffer::new().write();
        buffer.write_le_16(val1);
        buffer.write_le_16(val2);

        let mut buffer = buffer.read();

        assert_eq!(buffer.read_le_16(), val1);
        assert_eq!(buffer.read_le_16(), val2);
    }

    #[test]
    fn byte() {
        let val1 = 23;
        let val2 = 116;
        let mut buffer = ByteBuffer::new().write();
        buffer.write_byte(val1);
        buffer.write_byte(val2);

        let mut buffer = buffer.read();

        assert_eq!(buffer.read_byte(), val1);
        assert_eq!(buffer.read_byte(), val2);
    }
}
