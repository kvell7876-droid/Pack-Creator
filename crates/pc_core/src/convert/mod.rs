//! Turning an old girl's files into what the game reads now: Bedrock `.geo.json` models into `.strutmodel`, and `.animation.json` / `.scene.json` animations into
//! `.strutanim`, with cues made into markers and the cue tables made into event bindings.
//!
//! This is a port of Strut's converter (the TypeScript in the archived tool's `vendor/`) and is held to it byte for byte: the tests convert the game's own sources and
//! compare with the files the game ships.

pub mod animation;
pub mod events;
#[cfg(test)]
mod golden;
pub mod findings;
pub mod js;
pub mod model;
pub mod standalone;
pub mod writer;

pub use findings::{Finding, Findings, Severity, Where};
