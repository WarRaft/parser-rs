//! Building blocks for Warcraft III's binary data files (object data,
//! `war3map.w3i`, `.doo`, ...): a little-endian [`Reader`] that reports
//! errors with their byte offset, its twin [`Writer`], and [`Rawcode`],
//! the four-character id the game uses for everything.
//!
//! A format crate describes only its own layout and reads it field by
//! field; bounds checks, string decoding and error messages live here.

use std::fmt;

/// Why a binary file couldn't be read or written, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinError {
    /// Byte offset the problem was found at.
    pub offset: usize,
    pub message: String,
}

impl BinError {
    pub fn new(offset: usize, message: impl Into<String>) -> Self {
        BinError {
            offset,
            message: message.into(),
        }
    }
}

impl fmt::Display for BinError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at byte 0x{:X})", self.message, self.offset)
    }
}

impl std::error::Error for BinError {}

pub type BinResult<T> = Result<T, BinError>;

/// A four-character id (`'hfoo'`, `'AHbz'`), stored as its bytes in file
/// order. All zero bytes mean "none" in most formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Rawcode(pub [u8; 4]);

impl Rawcode {
    pub const NONE: Rawcode = Rawcode([0; 4]);

    pub fn is_none(self) -> bool {
        self == Rawcode::NONE
    }

    /// The integer JASS writes as `'hfoo'`: the first character is the
    /// most significant byte.
    pub fn to_int(self) -> u32 {
        u32::from_be_bytes(self.0)
    }

    pub fn from_int(value: u32) -> Self {
        Rawcode(value.to_be_bytes())
    }

    /// Whether [`Display`](fmt::Display) shows it as its four characters:
    /// all printable ASCII other than `'` and `\`.
    pub fn is_printable(self) -> bool {
        self.0
            .iter()
            .all(|&b| (0x20..0x7F).contains(&b) && b != b'\'' && b != b'\\')
    }

    /// Reads [`Display`](fmt::Display)'s output back: four printable
    /// characters, or `0x` and eight hex digits.
    pub fn parse(text: &str) -> Option<Self> {
        if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
            if hex.len() != 8 {
                return None;
            }
            return u32::from_str_radix(hex, 16).ok().map(Rawcode::from_int);
        }
        let bytes: [u8; 4] = text.as_bytes().try_into().ok()?;
        let code = Rawcode(bytes);
        code.is_printable().then_some(code)
    }
}

impl From<&[u8; 4]> for Rawcode {
    fn from(bytes: &[u8; 4]) -> Self {
        Rawcode(*bytes)
    }
}

/// The four characters (`hfoo`), or `0x` and the big-endian hex value
/// (`0x00000000`) when any byte isn't printable.
impl fmt::Display for Rawcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_printable() {
            self.0.iter().try_for_each(|&b| write!(f, "{}", b as char))
        } else {
            write!(f, "0x{:08X}", self.to_int())
        }
    }
}

/// Reads a little-endian file front to back. Every read checks bounds and
/// fails with the offset it happened at.
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    /// Current byte offset.
    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    /// An error at the current offset.
    pub fn error(&self, message: impl Into<String>) -> BinError {
        BinError::new(self.pos, message)
    }

    pub fn bytes(&mut self, n: usize) -> BinResult<&'a [u8]> {
        if self.remaining() < n {
            let left = self.remaining();
            return Err(self.error(format!(
                "unexpected end of file: need {n} bytes, {left} left"
            )));
        }
        let slice = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    pub fn array<const N: usize>(&mut self) -> BinResult<[u8; N]> {
        Ok(self.bytes(N)?.try_into().expect("length checked"))
    }

    /// Everything left, consuming it.
    pub fn rest(&mut self) -> &'a [u8] {
        let slice = &self.data[self.pos..];
        self.pos = self.data.len();
        slice
    }

    pub fn u8(&mut self) -> BinResult<u8> {
        Ok(self.bytes(1)?[0])
    }

    pub fn u16(&mut self) -> BinResult<u16> {
        self.array().map(u16::from_le_bytes)
    }

    pub fn u32(&mut self) -> BinResult<u32> {
        self.array().map(u32::from_le_bytes)
    }

    pub fn i32(&mut self) -> BinResult<i32> {
        self.array().map(i32::from_le_bytes)
    }

    pub fn f32(&mut self) -> BinResult<f32> {
        self.array().map(f32::from_le_bytes)
    }

    pub fn rawcode(&mut self) -> BinResult<Rawcode> {
        self.array().map(Rawcode)
    }

    /// A NUL-terminated UTF-8 string; the NUL is consumed.
    pub fn cstr(&mut self) -> BinResult<String> {
        let start = self.pos;
        let rest = &self.data[self.pos..];
        let Some(len) = rest.iter().position(|&b| b == 0) else {
            return Err(self.error("string runs to the end of the file"));
        };
        let text = std::str::from_utf8(&rest[..len])
            .map_err(|err| BinError::new(start + err.valid_up_to(), "string is not valid UTF-8"))?;
        self.pos += len + 1;
        Ok(text.to_string())
    }

    /// A `u32` item count, checked against what's left: each item takes
    /// at least `min_size` bytes, so a corrupt count fails here instead of
    /// allocating gigabytes.
    pub fn count(&mut self, min_size: usize) -> BinResult<usize> {
        let at = self.pos;
        let n = self.u32()? as usize;
        if n.saturating_mul(min_size.max(1)) > self.remaining() {
            return Err(BinError::new(
                at,
                format!(
                    "count {n} doesn't fit in the {} bytes left",
                    self.remaining()
                ),
            ));
        }
        Ok(n)
    }

    /// Reads a `u32` count, then that many items with `item`.
    pub fn list<T>(
        &mut self,
        min_size: usize,
        mut item: impl FnMut(&mut Self) -> BinResult<T>,
    ) -> BinResult<Vec<T>> {
        let n = self.count(min_size)?;
        (0..n).map(|_| item(self)).collect()
    }

    /// Fails unless the whole input was read.
    pub fn finish(&self) -> BinResult<()> {
        match self.remaining() {
            0 => Ok(()),
            1 => Err(self.error("1 extra byte after the end of the data")),
            n => Err(self.error(format!("{n} extra bytes after the end of the data"))),
        }
    }
}

/// Builds a little-endian file; the counterpart of [`Reader`].
#[derive(Default)]
pub struct Writer {
    data: Vec<u8>,
}

impl Writer {
    pub fn new() -> Self {
        Writer::default()
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }

    pub fn bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    pub fn u8(&mut self, value: u8) {
        self.data.push(value);
    }

    pub fn u16(&mut self, value: u16) {
        self.bytes(&value.to_le_bytes());
    }

    pub fn u32(&mut self, value: u32) {
        self.bytes(&value.to_le_bytes());
    }

    pub fn i32(&mut self, value: i32) {
        self.bytes(&value.to_le_bytes());
    }

    pub fn f32(&mut self, value: f32) {
        self.bytes(&value.to_le_bytes());
    }

    pub fn rawcode(&mut self, code: Rawcode) {
        self.bytes(&code.0);
    }

    /// A string and its NUL terminator. A string containing NUL can't be
    /// read back, so it's an error.
    pub fn cstr(&mut self, text: &str) -> BinResult<()> {
        if let Some(i) = text.bytes().position(|b| b == 0) {
            return Err(BinError::new(
                self.len() + i,
                "string contains a NUL character",
            ));
        }
        self.bytes(text.as_bytes());
        self.u8(0);
        Ok(())
    }

    /// A `u32` count; fails if `n` doesn't fit.
    pub fn count(&mut self, n: usize) -> BinResult<()> {
        let n = u32::try_from(n).map_err(|_| {
            BinError::new(self.len(), format!("{n} items don't fit in a u32 count"))
        })?;
        self.u32(n);
        Ok(())
    }
}

#[cfg(test)]
#[path = "bin.test.rs"]
mod tests;
