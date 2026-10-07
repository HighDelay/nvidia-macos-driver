use super::error::Error;
use spirv::Word;
use std::str;

pub(super) type Result<T> = std::result::Result<T, Error>;

const WORD_BYTES: usize = 4;

pub(super) struct Decoder<'a> {
    bytes: &'a [u8],
    offset: usize,
    limit: Option<usize>,
}

impl<'a> Decoder<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            offset: 0,
            limit: None,
        }
    }

    pub(super) fn offset(&self) -> usize {
        self.offset
    }

    pub(super) fn word(&mut self) -> Result<Word> {
        if let Some(left) = &mut self.limit {
            if *left == 0 {
                return Err(Error::LimitReached(self.offset));
            }
            *left -= 1;
        }
        let bytes = self
            .bytes
            .get(self.offset..self.offset + WORD_BYTES)
            .ok_or(Error::StreamExpected(self.offset))?;
        self.offset += WORD_BYTES;
        Ok(Word::from_le_bytes(
            bytes.try_into().expect("four-byte slice"),
        ))
    }

    pub(super) fn words(&mut self, count: usize) -> Result<Vec<Word>> {
        (0..count).map(|_| self.word()).collect()
    }

    pub(super) fn set_limit(&mut self, words: usize) {
        self.limit = Some(words);
    }

    pub(super) fn clear_limit(&mut self) {
        self.limit = None;
    }

    pub(super) fn limit_reached(&self) -> bool {
        self.limit == Some(0)
    }

    pub(super) fn id(&mut self) -> Result<Word> {
        self.word()
    }

    pub(super) fn string(&mut self) -> Result<String> {
        let end = self
            .limit
            .map_or(self.bytes.len(), |words| self.offset + words * WORD_BYTES);
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or(Error::StreamExpected(self.offset))?;
        let nul = bytes
            .iter()
            .position(|byte| *byte == 0)
            .ok_or(Error::LimitReached(end))?;
        let value = str::from_utf8(&bytes[..nul])
            .map_err(|error| Error::DecodeStringFailed(self.offset, error.to_string()))?
            .to_string();
        let words = nul / WORD_BYTES + 1;
        self.offset += words * WORD_BYTES;
        if let Some(left) = &mut self.limit {
            *left -= words;
        }
        Ok(value)
    }

    pub(super) fn bit32(&mut self) -> Result<u32> {
        self.word()
    }

    pub(super) fn bit64(&mut self) -> Result<u64> {
        let low = u64::from(self.word()?);
        let high = u64::from(self.word()?);
        Ok(low | high << 32)
    }

    pub(super) fn ext_inst_integer(&mut self) -> Result<u32> {
        self.word()
    }
}

include!("decode_generated.rs");
