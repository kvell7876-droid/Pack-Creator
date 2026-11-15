//! A little-endian byte sink and the string table both binary formats share. Strings are numbered in the order they are first used: that order is part of the bytes.

use std::collections::HashMap;

#[derive(Default)]
pub struct Out(pub Vec<u8>);

impl Out {
    pub fn u8(&mut self, value: u32) -> &mut Self {
        self.0.push((value & 0xFF) as u8);
        self
    }

    pub fn u16(&mut self, value: u32) -> &mut Self {
        self.0.extend(((value & 0xFFFF) as u16).to_le_bytes());
        self
    }

    pub fn i16(&mut self, value: i32) -> &mut Self {
        self.0.extend((value as i16).to_le_bytes());
        self
    }

    pub fn u32(&mut self, value: u32) -> &mut Self {
        self.0.extend(value.to_le_bytes());
        self
    }

    pub fn i32(&mut self, value: i32) -> &mut Self {
        self.0.extend(value.to_le_bytes());
        self
    }

    pub fn f32(&mut self, value: f64) -> &mut Self {
        self.0.extend((value as f32).to_le_bytes());
        self
    }

    pub fn vec3(&mut self, v: [f64; 3]) -> &mut Self {
        self.f32(v[0]).f32(v[1]).f32(v[2])
    }

    /// A string with its two-byte length in front.
    pub fn string(&mut self, value: &str) -> Result<&mut Self, String> {
        let bytes = value.as_bytes();

        if bytes.len() > 0xFFFF {
            return Err(format!("a string is too long for the format: {}...", value.chars().take(40).collect::<String>()));
        }

        self.u16(bytes.len() as u32);
        self.0.extend(bytes);
        Ok(self)
    }

    pub fn bytes(&mut self, value: &[u8]) -> &mut Self {
        self.0.extend(value);
        self
    }

    pub fn zeros(&mut self, count: usize) -> &mut Self {
        self.0.extend(std::iter::repeat(0u8).take(count));
        self
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

/// Strings in the order they were first used.
#[derive(Default, Clone)]
pub struct Strings {
    list: Vec<String>,
    index: HashMap<String, usize>,
}

impl Strings {
    pub fn intern(&mut self, value: &str) -> usize {
        if let Some(at) = self.index.get(value) {
            return *at;
        }

        let at = self.list.len();

        self.list.push(value.to_string());
        self.index.insert(value.to_string(), at);
        at
    }

    pub fn entries(&self) -> &[String] {
        &self.list
    }

    pub fn index_of(&self, value: &str, what: &str) -> Result<u32, String> {
        self.index.get(value).map(|i| *i as u32).ok_or_else(|| format!("encoding {what} \"{value}\", which is not in the string table"))
    }

    pub fn write(&self) -> Result<Out, String> {
        let mut out = Out::default();

        for value in &self.list {
            out.string(value)?;
        }

        Ok(out)
    }
}
