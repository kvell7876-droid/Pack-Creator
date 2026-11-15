//! A strict JSON reader that remembers where everything is.
//!
//! The checks have to say "this value, on this line" rather than "somewhere in the file", and `serde_json::Value` forgets positions. So files are read into
//! a tree of [`Node`]s that each carry the byte range they came from, and a [`LineIndex`] turns a byte offset into the line and column an editor uses.

/// One value and the bytes it was written in (`start..end`).
#[derive(Debug, Clone)]
pub struct Node {
    pub start: usize,
    pub end: usize,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Number { value: f64, integer: bool },
    Str(String),
    Array(Vec<Node>),
    Object(Vec<Member>),
}

/// One `"key": value` of an object, with where the key was written.
#[derive(Debug, Clone)]
pub struct Member {
    pub key: String,
    pub key_start: usize,
    pub key_end: usize,
    pub value: Node,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
}

impl Node {
    pub fn as_str(&self) -> Option<&str> {
        match &self.value {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[Member]> {
        match &self.value {
            Value::Object(members) => Some(members),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Node]> {
        match &self.value {
            Value::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match &self.value {
            Value::Number { value, .. } => Some(*value),
            _ => None,
        }
    }

    /// The member called `key`, if this is an object that has it (the first, when it has it twice).
    pub fn get(&self, key: &str) -> Option<&Member> {
        self.as_object()?.iter().find(|member| member.key == key)
    }

    /// What kind of value this is, in words for a message ("an object", "a number").
    pub fn describe(&self) -> &'static str {
        match &self.value {
            Value::Null => "null",
            Value::Bool(_) => "true or false",
            Value::Number { .. } => "a number",
            Value::Str(_) => "text",
            Value::Array(_) => "a list",
            Value::Object(_) => "an object",
        }
    }
}

const MAX_DEPTH: usize = 128;

/// Reads `text` as JSON, or says where it first goes wrong and why.
pub fn parse(text: &str) -> Result<Node, ParseError> {
    let mut reader = Reader { text, bytes: text.as_bytes(), at: 0 };

    reader.skip_space();

    if reader.at >= reader.bytes.len() {
        return Err(ParseError { message: "The file is empty. A JSON file needs at least {}.".into(), offset: 0 });
    }

    let node = reader.value(0)?;

    reader.skip_space();

    if reader.at < reader.bytes.len() {
        return Err(reader.error("Unexpected text after the end of the JSON. A file holds one value."));
    }

    Ok(node)
}

struct Reader<'a> {
    text: &'a str,
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError { message: message.into(), offset: self.at.min(self.bytes.len()) }
    }

    fn skip_space(&mut self) {
        while self.at < self.bytes.len() && matches!(self.bytes[self.at], b' ' | b'\t' | b'\n' | b'\r') {
            self.at += 1;
        }

        /* A byte order mark at the very start is harmless. */
        if self.at == 0 && self.bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            self.at = 3;
            self.skip_space();
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn value(&mut self, depth: usize) -> Result<Node, ParseError> {
        if depth > MAX_DEPTH {
            return Err(self.error("The JSON is nested too deeply."));
        }

        let start = self.at;

        match self.peek() {
            None => Err(self.error("The file ends where a value was expected.")),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => {
                let s = self.string()?;
                Ok(Node { start, end: self.at, value: Value::Str(s) })
            }
            Some(b't') => self.word("true", Value::Bool(true)),
            Some(b'f') => self.word("false", Value::Bool(false)),
            Some(b'n') => self.word("null", Value::Null),
            Some(b'-') | Some(b'0'..=b'9') => self.number(),
            Some(b'\'') => Err(self.error("Text is written in double quotes (\"like this\"), not single quotes.")),
            Some(b'/') => Err(self.error("Comments are not allowed in a JSON file.")),
            Some(other) => Err(self.error(format!("Unexpected '{}' where a value was expected.", other as char))),
        }
    }

    fn word(&mut self, word: &str, value: Value) -> Result<Node, ParseError> {
        let start = self.at;

        if self.text[self.at..].starts_with(word) {
            self.at += word.len();
            Ok(Node { start, end: self.at, value })
        } else {
            Err(self.error(format!("Unknown word. Did you mean {}?", word)))
        }
    }

    fn number(&mut self) -> Result<Node, ParseError> {
        let start = self.at;
        let mut integer = true;

        if self.peek() == Some(b'-') {
            self.at += 1;
        }

        match self.peek() {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => {
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.at += 1;
                }
            }
            _ => return Err(self.error("A number needs digits.")),
        }

        if self.peek() == Some(b'.') {
            integer = false;
            self.at += 1;

            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("A number needs digits after the point."));
            }

            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.at += 1;
            }
        }

        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            integer = false;
            self.at += 1;

            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.at += 1;
            }

            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("A number needs digits in its exponent."));
            }

            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.at += 1;
            }
        }

        let value: f64 = self.text[start..self.at].parse().map_err(|_| ParseError { message: "Not a valid number.".into(), offset: start })?;

        Ok(Node { start, end: self.at, value: Value::Number { value, integer } })
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.at += 1;

        let mut out = String::new();

        loop {
            let Some(byte) = self.peek() else {
                return Err(self.error("The text is not closed: add a closing \" before the end of the file."));
            };

            match byte {
                b'"' => {
                    self.at += 1;
                    return Ok(out);
                }
                b'\n' | b'\r' => return Err(self.error("The text is not closed on this line. Add the closing \".")),
                b'\\' => {
                    self.at += 1;

                    let Some(escape) = self.peek() else {
                        return Err(self.error("The file ends inside an escape."));
                    };

                    self.at += 1;

                    match escape {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => out.push(self.unicode()?),
                        other => {
                            self.at -= 1;
                            return Err(self.error(format!("'\\{}' is not a valid escape. Write a backslash as \\\\.", other as char)));
                        }
                    }
                }
                0x00..=0x1F => return Err(self.error("A control character must be escaped inside text.")),
                _ => {
                    let ch = self.text[self.at..].chars().next().unwrap();
                    out.push(ch);
                    self.at += ch.len_utf8();
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let end = self.at + 4;

        if end > self.bytes.len() {
            return Err(self.error("A \\u escape needs four hex digits."));
        }

        let digits = &self.text[self.at..end];
        let value = u32::from_str_radix(digits, 16).map_err(|_| self.error("A \\u escape needs four hex digits."))?;

        self.at = end;
        Ok(value)
    }

    fn unicode(&mut self) -> Result<char, ParseError> {
        let first = self.hex4()?;

        if (0xD800..0xDC00).contains(&first) {
            if self.text[self.at..].starts_with("\\u") {
                self.at += 2;

                let second = self.hex4()?;

                if (0xDC00..0xE000).contains(&second) {
                    return char::from_u32(0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)).ok_or_else(|| self.error("Not a valid character."));
                }
            }

            return Err(self.error("A high surrogate must be followed by a low one."));
        }

        char::from_u32(first).ok_or_else(|| self.error("Not a valid character."))
    }

    fn array(&mut self, depth: usize) -> Result<Node, ParseError> {
        let start = self.at;
        let mut items = Vec::new();

        self.at += 1;
        self.skip_space();

        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(Node { start, end: self.at, value: Value::Array(items) });
        }

        loop {
            self.skip_space();
            items.push(self.value(depth + 1)?);
            self.skip_space();

            match self.peek() {
                Some(b',') => {
                    self.at += 1;
                    self.skip_space();

                    if self.peek() == Some(b']') {
                        return Err(self.error("A trailing comma: remove the comma after the last item."));
                    }
                }
                Some(b']') => {
                    self.at += 1;
                    return Ok(Node { start, end: self.at, value: Value::Array(items) });
                }
                None => return Err(self.error("The list is not closed: add a ] before the end of the file.")),
                Some(_) => return Err(self.error("Expected a comma or a closing ] here. Items in a list are separated by commas.")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Node, ParseError> {
        let start = self.at;
        let mut members = Vec::new();

        self.at += 1;
        self.skip_space();

        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(Node { start, end: self.at, value: Value::Object(members) });
        }

        loop {
            self.skip_space();

            if self.peek() != Some(b'"') {
                return Err(self.error("Expected a name in double quotes here, such as \"id\"."));
            }

            let key_start = self.at;
            let key = self.string()?;
            let key_end = self.at;

            self.skip_space();

            if self.peek() != Some(b':') {
                return Err(self.error("Expected a colon after the name."));
            }

            self.at += 1;
            self.skip_space();

            let value = self.value(depth + 1)?;

            members.push(Member { key, key_start, key_end, value });
            self.skip_space();

            match self.peek() {
                Some(b',') => {
                    self.at += 1;
                    self.skip_space();

                    if self.peek() == Some(b'}') {
                        return Err(self.error("A trailing comma: remove the comma after the last entry."));
                    }
                }
                Some(b'}') => {
                    self.at += 1;
                    return Ok(Node { start, end: self.at, value: Value::Object(members) });
                }
                None => return Err(self.error("The object is not closed: add a } before the end of the file.")),
                Some(_) => return Err(self.error("Expected a comma or a closing } here. Entries are separated by commas.")),
            }
        }
    }
}

/// Turns a byte offset into the line and column an editor shows (both from 1, the column counted in UTF-16 units as Monaco does).
pub struct LineIndex<'a> {
    text: &'a str,
    starts: Vec<usize>,
}

impl<'a> LineIndex<'a> {
    pub fn new(text: &'a str) -> Self {
        let mut starts = vec![0];

        for (i, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(i + 1);
            }
        }

        LineIndex { text, starts }
    }

    pub fn position(&self, offset: usize) -> (u32, u32) {
        let offset = offset.min(self.text.len());
        let line = match self.starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next - 1,
        };
        let column = self.text[self.starts[line]..offset].encode_utf16().count();

        (line as u32 + 1, column as u32 + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_values_with_their_places() {
        let text = "{\n  \"id\": \"lucy\",\n  \"n\": [1, 2.5]\n}";
        let node = parse(text).unwrap();
        let id = node.get("id").unwrap();

        assert_eq!(id.value.as_str(), Some("lucy"));
        assert_eq!(&text[id.value.start..id.value.end], "\"lucy\"");

        let index = LineIndex::new(text);

        assert_eq!(index.position(id.key_start), (2, 3));
        assert_eq!(index.position(id.value.start), (2, 9));
    }

    #[test]
    fn says_where_and_why_it_went_wrong() {
        let error = parse("{ \"a\": 1, }").unwrap_err();

        assert!(error.message.contains("trailing comma"));
        assert_eq!(error.offset, 10);
        assert!(parse("{ \"a\": 'x' }").unwrap_err().message.contains("double quotes"));
        assert!(parse("{ \"a\": [1, 2 }").is_err());
        assert!(parse("").is_err());
        assert!(parse("{} {}").is_err());
    }

    #[test]
    fn counts_columns_like_the_editor() {
        let text = "\"é😀x\"";
        let index = LineIndex::new(text);

        /* é is one unit, the emoji two, so x is the fourth unit after the quote and the closing quote is the fifth. */
        assert_eq!(index.position(text.find('x').unwrap()), (1, 5));
    }

    #[test]
    fn numbers_know_whether_they_are_whole() {
        let node = parse("[1, 1.0, -2e3]").unwrap();
        let items = node.as_array().unwrap();

        assert!(matches!(items[0].value, Value::Number { integer: true, .. }));
        assert!(matches!(items[1].value, Value::Number { integer: false, .. }));
        assert!(matches!(items[2].value, Value::Number { integer: false, .. }));
    }
}
