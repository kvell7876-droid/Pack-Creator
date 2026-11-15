//! What a legacy animation event becomes, and the binding files that make it do something.
//!
//! Pleasure Horizons' rule, from its own tables: every token of a cue becomes a marker, verbatim, and the sounds and messages the old registry
//! (`keyframe_events/<girl>.json`, which only an old pack has: the game no longer reads it) defines become bindings on those markers, with the lines written in them. Dead entries (a key no animation uses) and inert markers (a token with no entry) produce nothing.

use super::animation::{LegacyEvent, RawEvent, EVENT_KIND_INSTRUCTION, EVENT_KIND_SOUND};
use super::findings::Severity;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A cue is split on commas and white space, lower cased, empty pieces dropped. A hyphen stays inside a token (the game splits messages on it itself).
pub fn split_cue_tokens(effect: &str) -> Vec<String> {
    effect.to_lowercase().split(|c: char| c == ',' || c.is_whitespace()).filter(|t| !t.is_empty()).map(String::from).collect()
}

/// A marker name uses letters, digits and `_ . / -`.
pub fn is_marker(token: &str) -> bool {
    !token.is_empty() && token.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'/' | b'-'))
}

#[derive(Debug, Clone, PartialEq)]
pub enum CueAction {
    Marker,
    Sound,
    Custom,
    LeaveOut,
}

/// What to do with one cue when the person overrides the suggestion.
#[derive(Debug, Clone)]
pub struct CueRule {
    pub action: CueAction,
    pub value: Option<String>,
    /// 0 lets the event type decide, 1 server, 2 client, 3 both.
    pub authority: u32,
}

pub type Mapping = HashMap<String, CueRule>;

/// Why a rule cannot be applied, in words, or None.
pub fn rule_problem(token: &str, rule: &CueRule) -> Option<String> {
    let value = rule.value.as_deref().map(str::trim).unwrap_or("");

    match rule.action {
        CueAction::Custom if value.is_empty() => Some(format!("the cue \"{token}\" is set to a custom event but has no event line (for example pleasurehorizons:fade 1.5)")),
        CueAction::Custom => {
            let head = value.split_whitespace().next().unwrap_or("");
            let ok = head.split_once(':').map_or(false, |(ns, path)| !ns.is_empty() && !path.is_empty() && ns.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')));

            (!ok).then(|| format!("the custom event \"{value}\" for the cue \"{token}\" should start with a namespaced type such as pleasurehorizons:fade"))
        }
        CueAction::Sound if !value.is_empty() && !value.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'/' | b'-')) => {
            Some(format!("the sound \"{value}\" for the cue \"{token}\" may only use letters, digits and _ . : / -"))
        }
        _ => None,
    }
}

/// Sound cues become one marker for each token, as `instruction` events, unless the person gave that token a rule. Particle and instruction events pass through unchanged.
/// `on_cue` hears every token found, `on_token` every token written as a marker.
pub fn marker_mapper<'a>(rules: &'a Mapping, tokens: &'a mut BTreeMap<String, usize>, cues: &'a mut BTreeMap<String, usize>) -> impl FnMut(&LegacyEvent, &mut Vec<(Severity, String)>) -> Vec<RawEvent> + 'a {
    move |event, report| {
        if event.kind != EVENT_KIND_SOUND {
            return vec![RawEvent { time: event.time, kind: event.kind.clone(), payload: event.payload.clone(), authority: 0 }];
        }

        let mut out = Vec::new();

        for token in split_cue_tokens(&event.payload) {
            if !is_marker(&token) {
                report.push((Severity::Lossy, format!("the cue token \"{token}\" is not a valid marker name (letters, digits, _ . / - only); skipped")));
                continue;
            }

            *cues.entry(token.clone()).or_default() += 1;

            let rule = rules.get(&token);
            let authority = rule.map_or(0, |r| r.authority);

            match rule {
                None => {
                    *tokens.entry(token.clone()).or_default() += 1;
                    out.push(RawEvent { time: event.time, kind: EVENT_KIND_INSTRUCTION.into(), payload: token, authority });
                }
                Some(rule) if rule.action == CueAction::Marker => {
                    *tokens.entry(token.clone()).or_default() += 1;
                    out.push(RawEvent { time: event.time, kind: EVENT_KIND_INSTRUCTION.into(), payload: token, authority });
                }
                Some(rule) => {
                    if let Some(problem) = rule_problem(&token, rule) {
                        report.push((Severity::Lossy, format!("{problem}; the cue is left out")));
                        continue;
                    }

                    let value = rule.value.as_deref().map(str::trim).unwrap_or("");

                    match rule.action {
                        CueAction::LeaveOut => {}
                        CueAction::Sound => out.push(RawEvent { time: event.time, kind: EVENT_KIND_SOUND.into(), payload: if value.is_empty() { token } else { value.to_string() }, authority }),
                        _ => out.push(RawEvent { time: event.time, kind: EVENT_KIND_INSTRUCTION.into(), payload: value.to_string(), authority }),
                    }
                }
            }
        }

        out
    }
}

// =================================================================
// The old registry: keyframe_events/<girl>.json (read from an old pack only)
// =================================================================

#[derive(Debug, Clone, Default)]
pub struct RegistryEntry {
    pub sounds: Vec<String>,
    pub random_sounds: Vec<String>,
    pub messages: Vec<String>,
    pub player_messages: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct GirlRegistry {
    pub girl_id: String,
    pub file: String,
    pub entries: HashMap<String, RegistryEntry>,
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value.and_then(Value::as_array).map(|list| list.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
}

pub fn parse_keyframe_events(json: &Value, file: &str) -> GirlRegistry {
    let girl_id = json.get("girl_id").and_then(Value::as_str).map(String::from).unwrap_or_else(|| file.trim_end_matches(".json").to_string());
    let mut entries = HashMap::new();

    for entry in json.get("events").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]) {
        let Some(key) = entry.get("key").and_then(Value::as_str) else { continue };

        /* A key written twice has the lines of both, as the game used to say them. */
        let slot: &mut RegistryEntry = entries.entry(key.to_lowercase()).or_default();

        slot.sounds.extend(strings(entry.get("sounds")));
        slot.random_sounds.extend(strings(entry.get("random_sounds")));
        slot.messages.extend(strings(entry.get("messages")));
        slot.player_messages.extend(strings(entry.get("player_messages")));
    }

    GirlRegistry { girl_id, file: file.to_string(), entries }
}

pub const MESSAGES_ACTION: &str = "pleasurehorizons:messages";
pub const PLAYER_MESSAGES_ACTION: &str = "pleasurehorizons:player_messages";

/// One girl's binding file for the tokens the animations use, and only those her registry defines something for: a token equal to one of her sound keys gets those
/// sounds, a token with a hyphen part equal to a message key gets that key's lines, a token equal to a player message key gets that key's lines. The lines are written in the binding.
pub fn binding_set_for(girl: &GirlRegistry, tokens: impl IntoIterator<Item = String>) -> Value {
    let unique: BTreeSet<String> = tokens.into_iter().collect();
    let mut bindings = Map::new();

    for token in unique {
        let mut binding = Map::new();
        let own = girl.entries.get(&token);

        if let Some(own) = own {
            if !own.sounds.is_empty() {
                binding.insert("sounds".into(), json!(own.sounds));
            }

            if !own.random_sounds.is_empty() {
                binding.insert("random_sounds".into(), json!(own.random_sounds));
            }
        }

        let lines: Vec<&String> = token.split('-').filter_map(|part| girl.entries.get(part)).flat_map(|e| e.messages.iter()).collect();

        if !lines.is_empty() {
            binding.insert(MESSAGES_ACTION.into(), json!(lines));
        }

        if let Some(own) = own.filter(|e| !e.player_messages.is_empty()) {
            binding.insert(PLAYER_MESSAGES_ACTION.into(), json!(own.player_messages));
        }

        if !binding.is_empty() {
            bindings.insert(token, Value::Object(binding));
        }
    }

    json!({ "bindings": bindings })
}
