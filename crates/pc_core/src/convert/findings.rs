//! What a conversion tells the person: every finding says where it happened.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Worth knowing, changes nothing.
    Info,
    /// The output differs from the input in a way that is deliberate and safe.
    Changed,
    /// Something in the input could not be carried over, or was replaced: look at it.
    Lossy,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Finding {
    pub severity: Severity,
    pub message: String,
    pub file: Option<String>,
    pub clip: Option<String>,
    pub bone: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct Findings {
    pub items: Vec<Finding>,
}

/// Where a finding is.
#[derive(Debug, Default, Clone)]
pub struct Where {
    pub file: Option<String>,
    pub clip: Option<String>,
    pub bone: Option<String>,
}

impl Findings {
    pub fn add(&mut self, severity: Severity, message: impl Into<String>, at: &Where) {
        self.items.push(Finding { severity, message: message.into(), file: at.file.clone(), clip: at.clip.clone(), bone: at.bone.clone() });
    }

    pub fn lossy(&self) -> usize {
        self.items.iter().filter(|f| f.severity == Severity::Lossy).count()
    }
}
