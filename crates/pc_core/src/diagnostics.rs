//! What the checks report: a problem, where it is, and when there is an obvious repair, the repair.

use crate::json_ast::LineIndex;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A repair the editor can offer: replace the problem's range with `replacement`.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct QuickFix {
    pub title: String,
    pub replacement: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    /// A short stable name for the kind of problem (`unknown-field`), for filtering and for tests.
    pub code: String,
    pub message: String,
    /// The file, relative to the pack root with `/`.
    pub file: String,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub fix: Option<QuickFix>,
}

/// Collects the problems of one file.
pub struct Sink<'a> {
    file: &'a str,
    index: LineIndex<'a>,
    pub items: Vec<Diagnostic>,
}

impl<'a> Sink<'a> {
    pub fn new(file: &'a str, text: &'a str) -> Self {
        Sink { file, index: LineIndex::new(text), items: Vec::new() }
    }

    pub fn report(&mut self, severity: Severity, code: &str, message: impl Into<String>, start: usize, end: usize, fix: Option<QuickFix>) {
        let (line, column) = self.index.position(start);
        let (end_line, end_column) = self.index.position(end.max(start));

        self.items.push(Diagnostic {
            severity,
            code: code.to_string(),
            message: message.into(),
            file: self.file.to_string(),
            line,
            column,
            end_line,
            end_column: if end_line == line && end_column == column { end_column + 1 } else { end_column },
            fix,
        });
    }

    pub fn error(&mut self, code: &str, message: impl Into<String>, start: usize, end: usize) {
        self.report(Severity::Error, code, message, start, end, None);
    }

    pub fn warn(&mut self, code: &str, message: impl Into<String>, start: usize, end: usize) {
        self.report(Severity::Warning, code, message, start, end, None);
    }

    pub fn info(&mut self, code: &str, message: impl Into<String>, start: usize, end: usize) {
        self.report(Severity::Info, code, message, start, end, None);
    }

    /// A warning with a repair: the range is replaced by `replacement`.
    pub fn warn_with_fix(&mut self, code: &str, message: impl Into<String>, start: usize, end: usize, title: impl Into<String>, replacement: String) {
        self.report(Severity::Warning, code, message, start, end, Some(QuickFix { title: title.into(), replacement }));
    }

    /// An error with a repair: the range is replaced by `replacement` (written as it appears in the file, quotes included).
    pub fn error_with_fix(&mut self, code: &str, message: impl Into<String>, start: usize, end: usize, title: impl Into<String>, replacement: String) {
        self.report(Severity::Error, code, message, start, end, Some(QuickFix { title: title.into(), replacement }));
    }
}

/// A problem that belongs to a whole file or to the pack, with no place in a text (a missing file).
pub fn file_level(severity: Severity, code: &str, message: impl Into<String>, file: &str) -> Diagnostic {
    Diagnostic { severity, code: code.to_string(), message: message.into(), file: file.to_string(), line: 1, column: 1, end_line: 1, end_column: 2, fix: None }
}
