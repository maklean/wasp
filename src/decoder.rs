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

    /// Returns the amount of bytes left to read.
    pub fn len(&self) -> usize {
        self.bytes.len() - self.pos
    }

    /// Returns whether we've reached EOF.
    pub fn eof(&self) -> bool {
        self.len() == 0
    }

    /// Reads a single byte from the data and advances by one.
    pub fn read_byte(&mut self) -> Result<u8, DecodeError> {
        if self.pos >= self.bytes.len() {
            return Err(DecodeError::UnexpectedEof);
        }

        let byte = self.bytes[self.pos];
        self.pos += 1;
        Ok(byte)
    }

    /// Reads `n` bytes from the data and advances by `n`.
    pub fn read_bytes(&mut self, n: usize) -> Result<&[u8], DecodeError> {
        if self.pos + n > self.bytes.len() {
            return Err(DecodeError::UnexpectedEof);
        }

        let slice = &self.bytes[self.pos..self.pos+n];
        self.pos += n;
        Ok(slice)
    }

    /// Reads a LEB128 encoded unsigned 32-bit integer.
    pub fn read_u32(&mut self) -> Result<u32, DecodeError> {
        Ok(self.read_uint(32)? as u32)
    }

    /// Reads a LEB128 encoded unsigned n-bit integer.
    fn read_uint(&mut self, bits: usize) -> Result<u64, DecodeError> {
        let mut n = 0u64;
        let mut bits_read: usize = 0;

        loop {
            let b = self.read_byte()?;
            let v = ((b & 0x7f) as u64) << bits_read; // get 7 bits and shift to correct position
            
            // If the high bit in the byte is 0, we're done reading.
            if b & 0x80 == 0 {
                let remaining_bits = bits - bits_read;

                // If the byte is trying to add more than what we can add, the number is malformed
                if b >= 1u8.checked_shl(remaining_bits as u32).unwrap_or(u8::MAX) {
                    return Err(DecodeError::MalformedInteger);
                }

                n += v;
                break;
            }

            n += v;
            bits_read += 7;

            // read too many bits
            if bits_read >= bits {
                return Err(DecodeError::MalformedInteger);
            }
        }

        Ok(n)
    }
}