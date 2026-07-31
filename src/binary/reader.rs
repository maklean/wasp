use crate::errors::DecodingError;

/// Reader into a sequence of bytes.
pub struct Reader<'a> {
    /// Sequence of bytes from a Wasm binary.
    bytes: &'a [u8],

    /// Current position in `bytes` sequence.
    pos: usize,
}

impl<'a> Reader<'a> {
    const WASM_MAGIC_NUMBER: &'static [u8; 4] = &[0x00, 0x61, 0x73, 0x6d];
    const WASM_1_0_SPEC_VERSION: &'static [u8; 4] = &[0x01, 0x00, 0x00, 0x00];

    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    /// Returns the current position of the decoder.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Returns the amount of bytes left to read.
    pub fn len(&self) -> usize {
        self.bytes.len() - self.pos
    }

    /// Returns whether we've reached EOF.
    pub fn eof(&self) -> bool {
        self.len() == 0
    }

    /// Reads the Wasm module header (magic number + version).
    pub fn read_wasm_header(&mut self) -> Result<(), DecodingError> {
        if self.read_bytes(Self::WASM_MAGIC_NUMBER.len())? != Self::WASM_MAGIC_NUMBER {
            return Err(DecodingError::InvalidMagicNumber);
        }

        if self.read_bytes(Self::WASM_1_0_SPEC_VERSION.len())? != Self::WASM_1_0_SPEC_VERSION {
            return Err(DecodingError::InvalidSpecificationVersion);
        }

        Ok(())
    }

    /// Reads a single byte from the data and advances by one.
    pub fn read_byte(&mut self) -> Result<u8, DecodingError> {
        if self.pos >= self.bytes.len() {
            return Err(DecodingError::UnexpectedEof { pos: self.bytes.len() });
        }

        let byte = self.bytes[self.pos];
        self.pos += 1;
        Ok(byte)
    }

    /// Reads `n` bytes from the data and advances by `n`.
    pub fn read_bytes(&mut self, n: usize) -> Result<&[u8], DecodingError> {
        if self.pos + n > self.bytes.len() {
            return Err(DecodingError::UnexpectedEof { pos: self.bytes.len() });
        }

        let slice = &self.bytes[self.pos..self.pos+n];
        self.pos += n;
        Ok(slice)
    }

    /// Peeks at the current byte, does not advance.
    pub fn peek_byte(&self) -> Result<u8, DecodingError> {
        if self.pos >= self.bytes.len() {
            return Err(DecodingError::UnexpectedEof { pos: self.bytes.len() });
        }

        Ok(self.bytes[self.pos])
    }

    /// Advances by one if the next byte matches the expected byte, otherwise the given error is returned.
    pub fn match_byte(&mut self, expect: u8, err: DecodingError) -> Result<(), DecodingError> {
        if self.read_byte()? == expect {
            Ok(())
        } else {
            Err(err)
        }
    }

    /// Reads a 32-bit floating point number.
    pub fn read_f32(&mut self) -> Result<f32, DecodingError> {
        let bytes: [u8; 4] = self.read_bytes(4)?
            .try_into()
            .map_err(|_| DecodingError::MalformedFloatingPoint { pos: self.pos - 4 })?;

        Ok(f32::from_le_bytes(bytes))
    }

    /// Reads a 64-bit floating point number.
    pub fn read_f64(&mut self) -> Result<f64, DecodingError> {
        let bytes: [u8; 8] = self.read_bytes(8)?
            .try_into()
            .map_err(|_| DecodingError::MalformedFloatingPoint { pos: self.pos - 8 })?;
        
        Ok(f64::from_le_bytes(bytes))
    }

    /// Reads a LEB128 encoded unsigned 32-bit integer.
    pub fn read_u32(&mut self) -> Result<u32, DecodingError> {
        Ok(self.read_uint(32)? as u32)
    }

    /// Reads a LEB128 encoded signed 32-bit integer.
    pub fn read_i32(&mut self) -> Result<i32, DecodingError> {
        Ok(self.read_sint(32)? as i32)
    }

    /// Reads a LEB128 encoded signed 64-bit integer.
    pub fn read_i64(&mut self) -> Result<i64, DecodingError> {
        Ok(self.read_sint(64)? as i64)
    }

    /// Reads a LEB128 encoded unsigned n-bit integer.
    fn read_uint(&mut self, bits: usize) -> Result<u64, DecodingError> {
        let mut n = 0u64;
        let mut bits_read: usize = 0;

        let pos = self.pos;

        loop {
            let b = self.read_byte()?;
            let v = ((b & 0x7f) as u64) << bits_read; // get 7 bits and shift to correct position
            
            // If the high bit in the byte is 0, we're done reading.
            if b & 0x80 == 0 {
                let remaining_bits = bits - bits_read;

                // If the byte is trying to add more than what we can add, the number is malformed
                if b >= 1u8.checked_shl(remaining_bits as u32).unwrap_or(u8::MAX) {
                    return Err(DecodingError::MalformedInteger { pos });
                }

                n += v;
                break;
            }

            n += v;
            bits_read += 7;

            // read too many bits
            if bits_read >= bits {
                return Err(DecodingError::MalformedInteger { pos });
            }
        }

        Ok(n)
    }

    /// Reads a LEB128 encoded signed n-bit integer.
    fn read_sint(&mut self, bits: usize) -> Result<i64, DecodingError> {
        let mut n = 0i64;
        let mut bits_read: usize = 0;

        let pos = self.pos;

        loop {
            let b = self.read_byte()?;
            let v = ((b & 0x7f) as i64) << bits_read;

            // If the high bit in the byte is 0, we're done reading.
            if b & 0x80 == 0 {
                let remaining_bits = bits - bits_read;

                // Check if it's a positive or negative number
                if b & 0x40 == 0 {
                    // positive: if the byte is trying to add more than what we can add, the number is malformed (-1 to disinclude the sign bit)
                    if b >= 1u8.checked_shl(remaining_bits as u32 - 1).unwrap_or(u8::MAX) {
                        return Err(DecodingError::MalformedInteger { pos });
                    }
                } else {
                    // negative: check if it's not "more negative" than the remaining_bits allow
                    if remaining_bits <= 8 && b < (0x80 - (1u8 << (remaining_bits as u32 - 1))) {
                        return Err(DecodingError::MalformedInteger { pos });
                    }
                }

                n += v;

                // sign extend if it's negative
                if b & 0x40 != 0 {
                    let shift = bits_read + 7;
                    if shift < 64 {
                        n += !0i64 << shift;
                    }
                }

                break;
            }

            n += v;
            bits_read += 7;

            if bits_read >= bits {
                return Err(DecodingError::MalformedInteger { pos });
            }
        }

        Ok(n)
    }
}