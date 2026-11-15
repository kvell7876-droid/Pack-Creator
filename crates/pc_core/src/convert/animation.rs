//! Legacy GeckoLib / Bedrock `.animation.json` (and `.scene.json`) into Strut's binary animation (`.strutanim`).
//!
//! A `.strutanim` stores the authoring value: rotation in degrees, scale 1:1. Position X is negated (a Bedrock file stores that offset already negated), so a literal becomes
//! `-x` and an expression `-(expr)`. Rotation X and Y literals are negated too, because the old loader negated them and the binary baker does not; rotation expressions
//! are left alone, as the old loader left them. What an animation event means in the new system is the event mapper's business ([`super::events`]).

use super::findings::{Findings, Severity, Where};
use super::js::{self, negated};
use super::writer::{Out, Strings};
use serde_json::Value;
use std::collections::HashMap;

pub const ANIMATION_VERSION: u32 = 100;
const HEADER_BYTES: usize = 32;
const KEY_FLAG_HAS_POST: u32 = 0x0001;
const KEY_FLAG_HAS_EASING: u32 = 0x0002;
const KEY_FLAG_VALUE_EXPRESSION: u32 = 0x0004;
const KEY_FLAG_POST_EXPRESSION: u32 = 0x0020;
const INTERPOLATION_MIXED: u32 = 255;

pub const EVENT_KIND_SOUND: &str = "sound";
pub const EVENT_KIND_PARTICLE: &str = "particle";
pub const EVENT_KIND_INSTRUCTION: &str = "instruction";
/// ASCII unit separator, between the fields of a particle event.
pub const EVENT_FIELD_SEPARATOR: char = '\u{1f}';

/// The easings the format knows by name, and which of them take arguments.
const EASINGS: [&str; 31] = [
    "linear", "step", "easeInQuad", "easeOutQuad", "easeInOutQuad", "easeInCubic", "easeOutCubic", "easeInOutCubic", "easeInQuart", "easeOutQuart", "easeInOutQuart", "easeInQuint",
    "easeOutQuint", "easeInOutQuint", "easeInSine", "easeOutSine", "easeInOutSine", "easeInExpo", "easeOutExpo", "easeInOutExpo", "easeInCirc", "easeOutCirc", "easeInOutCirc",
    "easeInBack", "easeOutBack", "easeInOutBack", "easeInElastic", "easeOutElastic", "easeInOutElastic", "easeInBounce", "easeOutBounce",
];

/// Whether the game knows this easing. It reads a name without regard to capitals ("easeinsine" is "easeInSine"), so so does this; "catmullrom" is an easing of its own.
fn is_easing(name: &str) -> bool {
    EASINGS.iter().any(|known| known.eq_ignore_ascii_case(name)) || name.eq_ignore_ascii_case("easeInOutBounce") || name.eq_ignore_ascii_case("catmullrom")
}

fn takes_arguments(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();

    lower.contains("back") || lower.contains("elastic") || lower.contains("bounce") || lower == "step"
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interpolation {
    Step = 0,
    Linear = 1,
    CatmullRom = 2,
    Bezier = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Property {
    Rotation = 0,
    Position = 1,
    Scale = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawValue {
    pub literals: [f64; 3],
    pub expressions: [Option<String>; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawKey {
    pub time: f64,
    pub interpolation: Interpolation,
    pub value: RawValue,
    pub post: Option<RawValue>,
    pub easing: Option<String>,
    pub easing_args: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct RawChannel {
    pub target: String,
    pub property: Property,
    /// The mode every key shares, or None when they differ.
    pub interpolation: Option<Interpolation>,
    pub keys: Vec<RawKey>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RawEvent {
    pub time: f64,
    pub kind: String,
    pub payload: String,
    pub authority: u32,
}

/// An event as the old file had it, before it is mapped.
#[derive(Debug, Clone)]
pub struct LegacyEvent {
    pub clip: String,
    pub time: f64,
    pub kind: String,
    pub payload: String,
}

#[derive(Debug, Clone)]
pub struct RawClip {
    pub name: String,
    pub duration: f64,
    /// 0 plays once, 1 loops, 2 holds the last frame.
    pub loop_mode: u32,
    pub channels: Vec<RawChannel>,
    pub events: Vec<RawEvent>,
}

#[derive(Debug, Clone)]
pub struct RawAnimation {
    pub flags: u32,
    pub fingerprint: i32,
    pub strings: Vec<String>,
    pub clips: Vec<RawClip>,
}

pub struct Converted {
    pub animation: RawAnimation,
    pub findings: Findings,
}

/// Turns one legacy event into the events to write, reporting what it needs to.
pub type EventMapper<'a> = dyn FnMut(&LegacyEvent, &mut Vec<(Severity, String)>) -> Vec<RawEvent> + 'a;

// =================================================================
// Encoding
// =================================================================

pub fn encode(animation: &RawAnimation) -> Result<Vec<u8>, String> {
    let mut strings = Strings::default();

    for s in &animation.strings {
        strings.intern(s);
    }

    let table = strings.write()?;
    let mut clips = Out::default();

    for clip in &animation.clips {
        clips.u16(strings.index_of(&clip.name, "clip name")?).f32(clip.duration).u8(clip.loop_mode).u16(clip.channels.len() as u32).u16(clip.events.len() as u32);

        for channel in &clip.channels {
            clips
                .u16(strings.index_of(&channel.target, "channel target")?)
                .u8(0)
                .u8(channel.property as u32)
                .u8(channel.interpolation.map_or(INTERPOLATION_MIXED, |i| i as u32))
                .u16(channel.keys.len() as u32);

            for key in &channel.keys {
                write_key(&mut clips, key, &strings)?;
            }
        }

        for event in &clip.events {
            clips.f32(event.time).u16(strings.index_of(&event.kind, "event kind")?).u16(strings.index_of(&event.payload, "event payload")?).u8(event.authority);
        }
    }

    let mut header = Out::default();

    header.bytes(b"STAM").u16(ANIMATION_VERSION).u16(animation.flags).u16(animation.strings.len() as u32).u32(table.len() as u32).u16(animation.clips.len() as u32).i32(animation.fingerprint).zeros(12);

    if header.len() != HEADER_BYTES {
        return Err(format!("wrote a {}-byte animation header, expected {HEADER_BYTES}", header.len()));
    }

    let mut out = Out::default();

    out.bytes(&header.0).bytes(&table.0).bytes(&clips.0);
    Ok(out.0)
}

fn expression_flags(value: &RawValue, bit: u32) -> u32 {
    (0..3).filter(|i| value.expressions[*i].is_some()).fold(0, |flags, i| flags | (bit << i))
}

fn write_value(out: &mut Out, value: &RawValue, strings: &Strings) -> Result<(), String> {
    for i in 0..3 {
        match &value.expressions[i] {
            Some(expression) => {
                out.u16(strings.index_of(expression, "a value expression")?);
            }
            None => {
                out.f32(value.literals[i]);
            }
        }
    }

    Ok(())
}

fn write_key(out: &mut Out, key: &RawKey, strings: &Strings) -> Result<(), String> {
    out.f32(key.time).u8(key.interpolation as u32);

    let mut flags = 0;

    if key.post.is_some() {
        flags |= KEY_FLAG_HAS_POST;
    }

    if key.easing.is_some() {
        flags |= KEY_FLAG_HAS_EASING;
    }

    flags |= expression_flags(&key.value, KEY_FLAG_VALUE_EXPRESSION);

    if let Some(post) = &key.post {
        flags |= expression_flags(post, KEY_FLAG_POST_EXPRESSION);
    }

    out.u16(flags);
    write_value(out, &key.value, strings)?;

    if let Some(post) = &key.post {
        write_value(out, post, strings)?;
    }

    if let Some(easing) = &key.easing {
        out.u16(strings.index_of(easing, "an easing name")?).u8(key.easing_args.len() as u32);

        for arg in &key.easing_args {
            out.f32(*arg);
        }
    }

    Ok(())
}

// =================================================================
// Conversion
// =================================================================

pub struct Options<'a> {
    pub file: Option<String>,
    /// From the model this file belongs to; 0 when unknown (a mismatch is only a warning in the game).
    pub fingerprint: i32,
    /// What an event becomes. None passes every event through unchanged.
    pub map_event: Option<&'a mut EventMapper<'a>>,
}

pub fn convert(root: &Value, mut options: Options) -> Converted {
    let mut findings = Findings::default();
    let here = Where { file: options.file.clone(), ..Default::default() };
    let mut strings = Strings::default();
    let mut clips = Vec::new();

    let Some(animations) = root.get("animations").and_then(Value::as_object) else {
        findings.add(Severity::Lossy, "no \"animations\" object; nothing converted", &here);

        return Converted { animation: RawAnimation { flags: 0, fingerprint: options.fingerprint, strings: Vec::new(), clips }, findings };
    };

    for (name, data) in js::entries(animations) {
        clips.push(convert_clip(name, data, &mut strings, &mut findings, &mut options));
    }

    Converted { animation: RawAnimation { flags: 0, fingerprint: options.fingerprint, strings: strings.entries().to_vec(), clips }, findings }
}

const PROPERTIES: [(&str, Property); 3] = [("rotation", Property::Rotation), ("position", Property::Position), ("scale", Property::Scale)];

fn convert_clip(name: &str, data: &Value, strings: &mut Strings, findings: &mut Findings, options: &mut Options) -> RawClip {
    let at = Where { file: options.file.clone(), clip: Some(name.to_string()), bone: None };
    let mut channels = Vec::new();
    let mut last_time: f64 = 0.0;
    let null = Value::Null;

    strings.intern(name);

    if let Some(bones) = data.get("bones").and_then(Value::as_object) {
        for (bone, bone_data) in js::entries(bones) {
            let bone_at = Where { bone: Some(bone.clone()), ..at.clone() };

            for (json, property) in PROPERTIES {
                let Some(value) = bone_data.get(json) else { continue };
                let Some(channel) = convert_channel(bone, property, value, strings, findings, &bone_at) else { continue };

                for key in &channel.keys {
                    last_time = last_time.max(key.time);
                }

                channels.push(channel);
            }

            if let Some(object) = bone_data.as_object() {
                for other in object.keys() {
                    if !PROPERTIES.iter().any(|(json, _)| json == other) {
                        findings.add(Severity::Lossy, format!("bone data \"{other}\" is not carried over (only rotation, position and scale are)"), &bone_at);
                    }
                }
            }
        }
    }

    let events = convert_events(name, data, strings, findings, options);

    for event in &events {
        last_time = last_time.max(event.time);
    }

    if let Some(object) = data.as_object() {
        for other in object.keys() {
            if !["bones", "loop", "animation_length", "sound_effects", "particle_effects", "timeline", "override_previous_animation"].contains(&other.as_str()) {
                findings.add(Severity::Lossy, format!("clip data \"{other}\" is not carried over"), &at);
            }
        }
    }

    let duration = data.get("animation_length").and_then(Value::as_f64).unwrap_or(last_time);
    let loop_mode = match data.get("loop") {
        Some(Value::Bool(true)) => 1,
        Some(Value::String(s)) if s == "hold_on_last_frame" => 2,
        _ => 0,
    };

    let _ = null;
    RawClip { name: name.to_string(), duration, loop_mode, channels, events }
}

fn is_constant(data: &Value) -> bool {
    match data {
        Value::Array(_) | Value::Number(_) | Value::String(_) => true,
        Value::Object(map) => !map.is_empty() && map.keys().all(|key| !js::finite(key)),
        _ => false,
    }
}

fn convert_channel(bone: &str, property: Property, data: &Value, strings: &mut Strings, findings: &mut Findings, at: &Where) -> Option<RawChannel> {
    strings.intern(bone);

    let mut keys: Vec<RawKey> = Vec::new();

    if is_constant(data) {
        let vector = data.get("vector").filter(|v| !v.is_null()).unwrap_or(data);
        let value = value_of(vector, property, strings, findings, at, "at time 0")?;

        keys.push(make_key(0.0, value, None, None, Vec::new(), Interpolation::Linear));
    } else if let Some(map) = data.as_object() {
        let mut times: Vec<(f64, RawKey)> = Vec::new();

        for (time_text, entry) in js::entries(map) {
            let time = js::number(time_text);

            if !time.is_finite() {
                findings.add(Severity::Lossy, format!("\"{time_text}\" is not a time; that entry is skipped"), at);
                continue;
            }

            let Some(key) = convert_key(time, entry, property, strings, findings, at) else { continue };

            match times.iter_mut().find(|(t, _)| *t == time) {
                Some(slot) => {
                    findings.add(Severity::Lossy, format!("two keyframes at {}s; the later one wins", js_text(time)), at);
                    slot.1 = key;
                }
                None => times.push((time, key)),
            }
        }

        times.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        keys.extend(times.into_iter().map(|(_, key)| key));
    } else {
        findings.add(Severity::Lossy, format!("{} has an unreadable value; the channel is skipped", property_name(property)), at);
        return None;
    }

    if keys.is_empty() {
        return None;
    }

    let first = keys[0].interpolation;
    let interpolation = if keys.iter().all(|k| k.interpolation == first) { Some(first) } else { None };

    Some(RawChannel { target: bone.to_string(), property, interpolation, keys })
}

fn property_name(property: Property) -> &'static str {
    match property {
        Property::Rotation => "rotation",
        Property::Position => "position",
        Property::Scale => "scale",
    }
}

/// A number written the way JavaScript writes it in a message (5 and not 5.0).
fn js_text(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", n as i64) } else { format!("{n}") }
}

fn unwrap_vector(value: &Value) -> &Value {
    match value {
        Value::Object(map) => map.get("vector").unwrap_or(value),
        _ => value,
    }
}

fn convert_key(time: f64, entry: &Value, property: Property, strings: &mut Strings, findings: &mut Findings, at: &Where) -> Option<RawKey> {
    let when = format!("at {}s", js_text(time));

    if matches!(entry, Value::Array(_) | Value::Number(_) | Value::String(_)) {
        let value = value_of(entry, property, strings, findings, at, &when)?;

        return Some(make_key(time, value, None, None, Vec::new(), Interpolation::Linear));
    }

    let Some(object) = entry.as_object() else {
        findings.add(Severity::Lossy, format!("keyframe {when} is unreadable; skipped"), at);
        return None;
    };

    /* pre and post are a vector or { "vector": [...] } (the real files use the second). */
    let pre = object.get("pre").and_then(|v| value_of(unwrap_vector(v), property, strings, findings, at, &when));
    let post = object.get("post").and_then(|v| value_of(unwrap_vector(v), property, strings, findings, at, &when));
    let plain = object.get("vector").and_then(|v| value_of(v, property, strings, findings, at, &when));
    let value = pre.clone().or(plain).or(post.clone());

    let Some(value) = value else {
        findings.add(Severity::Lossy, format!("keyframe {when} has no value; skipped"), at);
        return None;
    };

    for other in object.keys() {
        if !["vector", "pre", "post", "easing", "easingArgs", "lerp_mode"].contains(&other.as_str()) {
            findings.add(Severity::Lossy, format!("keyframe field \"{other}\" {when} is not carried over"), at);
        }
    }

    let mut easing: Option<String> = None;

    if let Some(name) = object.get("easing").and_then(Value::as_str) {
        if !name.eq_ignore_ascii_case("linear") && !name.eq_ignore_ascii_case("none") {
            if is_easing(name) {
                easing = Some(name.to_string());
                strings.intern(name);
            } else {
                findings.add(Severity::Lossy, format!("unknown easing \"{name}\" {when}; treated as linear"), at);
            }
        }
    }

    let easing_args: Vec<f64> = match (&easing, object.get("easingArgs").and_then(Value::as_array)) {
        (Some(name), Some(list)) if takes_arguments(name) => list.iter().filter_map(Value::as_f64).collect(),
        _ => Vec::new(),
    };
    let interpolation = if object.get("lerp_mode").and_then(Value::as_str) == Some("catmullrom") { Interpolation::CatmullRom } else { Interpolation::Linear };
    let outgoing = if pre.is_some() && post.is_some() { post } else { None };

    Some(make_key(time, value, outgoing, easing, easing_args, interpolation))
}

fn make_key(time: f64, value: RawValue, post: Option<RawValue>, easing: Option<String>, easing_args: Vec<f64>, interpolation: Interpolation) -> RawKey {
    RawKey { time, interpolation, value, post, easing, easing_args }
}

/// One vector of three components. A number is a literal; a string that reads as a number is a literal; an easing name is data that was never a value (0, reported);
/// anything else is a Molang expression. Position X is negated.
fn value_of(raw: &Value, property: Property, strings: &mut Strings, findings: &mut Findings, at: &Where, when: &str) -> Option<RawValue> {
    let name = property_name(property);
    let components: Vec<Option<&Value>> = match raw {
        Value::Array(list) => {
            if list.len() != 3 {
                findings.add(Severity::Lossy, format!("a {name} value {when} has {} components, expected 3; missing ones are 0", list.len()), at);
            }

            vec![list.first(), list.get(1), list.get(2)]
        }
        Value::Number(_) | Value::String(_) => vec![Some(raw); 3],
        _ => {
            findings.add(Severity::Lossy, format!("a {name} value {when} is unreadable; skipped"), at);
            return None;
        }
    };

    let mut literals = [0.0; 3];
    let mut expressions: [Option<String>; 3] = [None, None, None];

    for (i, axis) in ["x", "y", "z"].iter().enumerate() {
        let negate = (property == Property::Position && i == 0) || (property == Property::Rotation && i < 2);
        let negate_expression = property == Property::Position && i == 0;
        let apply = |n: f64| if negate { negated(n) } else { n };

        match components[i] {
            Some(Value::Number(n)) => literals[i] = apply(n.as_f64().unwrap_or(0.0)),
            Some(Value::String(text)) => {
                let text = text.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
                let stray: String = text.replace('`', "");

                if !text.is_empty() && js::finite(text) {
                    literals[i] = apply(js::number(text));
                } else if let Some(pasted) = pasted_number(text) {
                    /* An easing name pasted after a number ("12.45+easeInOutSine"): the old loader dropped the whole clip. */
                    findings.add(Severity::Lossy, format!("the value \"{text}\" {when} has an easing name pasted after the number; the number {} is used for {axis}", pasted.0), at);
                    literals[i] = apply(js::number(&pasted.0));
                } else if text == "." {
                    findings.add(Severity::Changed, format!("the value \".\" {when} is not a number; written as 0 for {axis}"), at);
                } else if stray != text && !stray.is_empty() && js::finite(&stray) {
                    findings.add(Severity::Changed, format!("the value \"{text}\" {when} has a stray character; read as {} for {axis}", js_text(js::number(&stray))), at);
                    literals[i] = apply(js::number(&stray));
                } else if is_easing(text) {
                    findings.add(Severity::Lossy, format!("the easing name \"{text}\" was used as the {axis} value {when}; written as 0"), at);
                } else if text.is_empty() {
                    literals[i] = 0.0;
                } else {
                    let expression = if negate_expression { format!("-({text})") } else { text.to_string() };

                    strings.intern(&expression);
                    expressions[i] = Some(expression);
                }
            }
            None | Some(Value::Null) => literals[i] = 0.0,
            Some(_) => findings.add(Severity::Lossy, format!("the {axis} value {when} is not a number or an expression; written as 0"), at),
        }
    }

    Some(RawValue { literals, expressions })
}

/// `^(-?\d+(?:\.\d+)?)\s*\+\s*(ease[A-Za-z]*)$`: a number with an easing name pasted after it.
fn pasted_number(text: &str) -> Option<(String, String)> {
    let plus = text.find('+')?;
    let (left, right) = (text[..plus].trim_end(), text[plus + 1..].trim_start());
    let digits = left.strip_prefix('-').unwrap_or(left);
    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w, Some(f)),
        None => (digits, None),
    };
    let numeric = !whole.is_empty() && whole.bytes().all(|b| b.is_ascii_digit()) && fraction.map_or(true, |f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()));
    let name = right.strip_prefix("ease").map_or(false, |rest| rest.bytes().all(|b| b.is_ascii_alphabetic()));

    (numeric && name).then(|| (left.to_string(), right.to_string()))
}

// =================================================================
// Events
// =================================================================

fn convert_events(clip: &str, data: &Value, strings: &mut Strings, findings: &mut Findings, options: &mut Options) -> Vec<RawEvent> {
    let at = Where { file: options.file.clone(), clip: Some(clip.to_string()), bone: None };
    let mut legacy: Vec<LegacyEvent> = Vec::new();

    collect(data.get("sound_effects"), findings, &at, |time, point| {
        if let Some(effect) = point.get("effect").and_then(Value::as_str).filter(|e| !e.is_empty()) {
            legacy.push(LegacyEvent { clip: clip.to_string(), time, kind: EVENT_KIND_SOUND.into(), payload: effect.to_string() });
        }
    });

    collect(data.get("particle_effects"), findings, &at, |time, point| {
        if let Some(effect) = point.get("effect").and_then(Value::as_str).filter(|e| !e.is_empty()) {
            let text = |key: &str| point.get(key).and_then(Value::as_str).filter(|s| !s.is_empty());
            let locator = text("locator").unwrap_or("");
            let script = text("pre_effect_script").or_else(|| text("script")).unwrap_or("");
            let payload = [effect, locator, script].join(&EVENT_FIELD_SEPARATOR.to_string());

            legacy.push(LegacyEvent { clip: clip.to_string(), time, kind: EVENT_KIND_PARTICLE.into(), payload });
        }
    });

    if let Some(timeline) = data.get("timeline").and_then(Value::as_object) {
        for (time_text, script) in js::entries(timeline) {
            let time = js::number(time_text);
            let text = match script {
                Value::Array(list) => Some(list.iter().map(|v| match v { Value::String(s) => s.clone(), other => other.to_string() }).collect::<Vec<_>>().join("\n")),
                Value::String(s) => Some(s.clone()),
                _ => None,
            };

            if let (true, Some(text)) = (time.is_finite(), text) {
                if !text.trim().is_empty() {
                    legacy.push(LegacyEvent { clip: clip.to_string(), time, kind: EVENT_KIND_INSTRUCTION.into(), payload: text });
                }
            }
        }
    }

    let mut events: Vec<RawEvent> = Vec::new();

    for event in &legacy {
        let mut reports: Vec<(Severity, String)> = Vec::new();
        let mapped = match options.map_event.as_mut() {
            Some(map) => map(event, &mut reports),
            None => vec![RawEvent { time: event.time, kind: event.kind.clone(), payload: event.payload.clone(), authority: 0 }],
        };

        for (severity, message) in reports {
            findings.add(severity, format!("at {}s: {message}", js_text(event.time)), &at);
        }

        for m in mapped {
            strings.intern(&m.kind);
            strings.intern(&m.payload);
            events.push(m);
        }
    }

    /* Stable: events at one time keep the order they were collected in. */
    events.sort_by(|a, b| a.time.partial_cmp(&b.time).unwrap_or(std::cmp::Ordering::Equal));
    events
}

/// `{ "0.5": {...} }` or `{ "0.5": [{...}, {...}] }`: calls `each` once for each point.
fn collect(table: Option<&Value>, findings: &mut Findings, at: &Where, mut each: impl FnMut(f64, &Value)) {
    let Some(table) = table.filter(|t| !t.is_null()) else { return };
    let Some(map) = table.as_object() else {
        findings.add(Severity::Lossy, "an event table is not an object; skipped", at);
        return;
    };

    for (time_text, entry) in js::entries(map) {
        let time = js::number(time_text);

        if !time.is_finite() {
            findings.add(Severity::Lossy, format!("\"{time_text}\" is not a time; that event is skipped"), at);
            continue;
        }

        match entry {
            Value::Array(points) => points.iter().for_each(|p| each(time, p)),
            other => each(time, other),
        }
    }
}

/// The fingerprint a model gives the animations authored against it, looked up by the id a file is named for.
pub fn fingerprints(models: &HashMap<String, i32>, id: &str) -> i32 {
    models.get(id).copied().unwrap_or(0)
}

#[cfg(test)]
mod easing_tests {
    use super::*;

    #[test]
    fn easing_names_are_read_without_regard_to_capitals_like_the_game_does() {
        assert!(is_easing("easeInSine") && is_easing("easeinsine") && is_easing("EASEINOUTBOUNCE") && is_easing("catmullrom"));
        assert!(!is_easing("easeSideways"));
        assert!(takes_arguments("easeinback") && takes_arguments("easeOutElastic") && takes_arguments("STEP"));
        assert!(!takes_arguments("easeInSine"));
    }

    #[test]
    fn a_lower_case_easing_is_kept_with_its_arguments() {
        let doc = serde_json::json!({ "animations": { "animation.a.x": { "animation_length": 1.0, "bones": { "root": { "rotation": {
            "0": { "vector": [0, 0, 0] },
            "1": { "vector": [90, 0, 0], "easing": "easeinback", "easingArgs": [2.0] }
        } } } } } });
        let converted = convert(&doc, Options { file: None, fingerprint: 0, map_event: None });
        let key = &converted.animation.clips[0].channels[0].keys[1];

        assert_eq!(key.easing.as_deref(), Some("easeinback"));
        assert_eq!(key.easing_args, vec![2.0]);
        assert!(converted.findings.items.iter().all(|f| !f.message.contains("unknown easing")));
    }
}
