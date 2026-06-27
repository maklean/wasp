use crate::errors::DecodeError;

pub struct Decoder<'a> {
    /// Sequence of bytes from a Wasm binary.
    bytes: &'a [u8],

    /// Current position in `bytes` sequence.
    pos: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub fn read_byte(&mut self) -> Result<u8, DecodeError> {
        if self.pos >= self.bytes.len() {
            return Err(DecodeError::UnexpectedEof);
        }

        let byte = self.bytes[self.pos];
        self.pos += 1;
        Ok(byte)
    }

    pub fn read_bytes(&mut self, n: usize) -> Result<&[u8], DecodeError> {
        if self.pos + n > self.bytes.len() {
            return Err(DecodeError::UnexpectedEof);
        }

        let slice = &self.bytes[self.pos..self.pos+n];
        self.pos += n;
        Ok(slice)
    }
}