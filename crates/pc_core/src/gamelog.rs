//! Reading the game's log after a try. Minecraft writes `logs/latest.log`; Pleasure Horizons puts what is wrong with a girl, scene, trait or dialogue file in it, and Strut
//! says what asset sync offered. This finds those lines, so the Pack Creator can show them next to the files they are about.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Entry {
    pub time: String,
    /// WARN, ERROR or FATAL.
    pub level: String,
    /// The part of the game that wrote it, as the log names it (`Pleasure Horizons`, `Strut`, `Minecraft`).
    pub source: String,
    pub message: String,
    /// The first resource id of a data file the message names (`pleasurehorizons:girls/zoe.json`).
    pub resource: Option<String>,
    /// How many times the same message was written.
    pub count: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct AssetOffer {
    pub time: String,
    pub files: u64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Report {
    pub entries: Vec<Entry>,
    /// The last time asset sync said what it offers.
    pub offer: Option<AssetOffer>,
    pub lines: usize,
}

/// A log line is `[12:34:56] [Render thread/WARN] (Source) message`.
struct Line<'a> {
    time: &'a str,
    level: &'a str,
    source: &'a str,
    message: &'a str,
}

fn split_line(line: &str) -> Option<Line<'_>> {
    let rest = line.strip_prefix('[')?;
    let (time, rest) = rest.split_once("] [")?;

    if time.len() != 8 || !time.bytes().all(|b| b.is_ascii_digit() || b == b':') {
        return None;
    }

    let (thread, rest) = rest.split_once("] ")?;
    let level = thread.rsplit('/').next()?;
    let rest = rest.strip_prefix('(')?;
    let (source, message) = rest.split_once(") ").or_else(|| rest.split_once(')'))?;

    Some(Line { time, level, source, message })
}

/// The first resource id in a message that names a data file: `namespace:folder/name.json`.
pub fn resource_of(message: &str) -> Option<String> {
    for word in message.split(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '/' | '.' | '-'))) {
        let word = word.trim_end_matches('.');

        if word.ends_with(".json") && word.contains(':') && word.contains('/') {
            return Some(word.to_string());
        }
    }

    None
}

fn number_after(text: &str, marker: &str) -> Option<u64> {
    let at = text.find(marker)? + marker.len();
    let digits: String = text[at..].chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();

    digits.parse().ok()
}

pub fn parse(text: &str) -> Report {
    let mut report = Report::default();
    let mut open: Option<usize> = None;
    let mut continuation = 0;

    for raw in text.lines() {
        report.lines += 1;

        let Some(line) = split_line(raw) else {
            /* A line without a time goes with the entry before it (a message that goes on over several lines), but a stack trace is not worth keeping. */
            if let Some(index) = open {
                let trimmed = raw.trim_end();

                if !trimmed.trim().is_empty() && !trimmed.trim_start().starts_with("at ") && !trimmed.starts_with("Caused by") && continuation < 24 {
                    let entry: &mut Entry = &mut report.entries[index];

                    entry.message.push('\n');
                    entry.message.push_str(trimmed);
                    continuation += 1;

                    if entry.resource.is_none() {
                        entry.resource = resource_of(trimmed);
                    }
                }
            }

            continue;
        };

        open = None;
        continuation = 0;

        if line.source == "Strut" && line.message.contains("[Strut asset sync] offering") {
            report.offer = Some(AssetOffer { time: line.time.to_string(), files: number_after(line.message, "offering").unwrap_or(0), bytes: number_after(line.message, "file(s),").unwrap_or(0) });
            continue;
        }

        let level = line.level;

        if !matches!(level, "WARN" | "ERROR" | "FATAL") {
            continue;
        }

        let about_us = line.source.contains("Pleasure Horizons") || line.source == "Strut" || line.message.contains("pleasurehorizons:") || line.message.contains("[Strut");

        if !about_us {
            continue;
        }

        let message = line.message.trim_end().to_string();

        report.entries.push(Entry { time: line.time.to_string(), level: level.to_string(), source: line.source.to_string(), resource: resource_of(&message), message, count: 1 });
        open = Some(report.entries.len() - 1);
    }

    /* The same message written again and again is shown once with how often. */
    let mut merged: Vec<Entry> = Vec::new();

    for entry in report.entries.drain(..) {
        match merged.iter_mut().find(|m| m.level == entry.level && m.source == entry.source && m.message == entry.message) {
            Some(existing) => existing.count += 1,
            None => merged.push(entry),
        }
    }

    report.entries = merged;
    report
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "[17:01:29] [main/INFO] (Minecraft) Setting user\n\
[17:01:47] [Render thread/ERROR] (Pleasure Horizons) \n\
  ERROR pleasurehorizons:girls/wandering_merchant.json at components.pleasurehorizons:taste.tame_item: The tame item is already used.\n\
  WARN pleasurehorizons:scenes/x.json: something\n\
\tat com.example.Thing.run(Thing.java:1)\n\
[17:01:47] [Render thread/INFO] (Strut) [Strut asset sync] offering 8 file(s), 1327142 bytes (1327142 held in memory)\n\
[17:01:48] [Render thread/WARN] (Minecraft) Missing sound for event: pleasurehorizons:misc.flap\n\
[17:01:48] [Render thread/WARN] (Minecraft) Missing sound for event: pleasurehorizons:misc.flap\n\
[17:01:49] [Render thread/WARN] (Minecraft) Missing metadata in pack file/CustomModels.zip\n\
[17:01:50] [Server thread/WARN] (Strut) [Strut asset sync] a client was slow\n";

    #[test]
    fn finds_what_the_game_says_about_a_pack() {
        let report = parse(LOG);

        assert_eq!(report.offer, Some(AssetOffer { time: "17:01:47".into(), files: 8, bytes: 1327142 }));
        assert_eq!(report.entries.len(), 3, "{:?}", report.entries);

        let girl = &report.entries[0];

        assert_eq!(girl.level, "ERROR");
        assert_eq!(girl.resource.as_deref(), Some("pleasurehorizons:girls/wandering_merchant.json"));
        assert!(girl.message.contains("tame item") && girl.message.contains("scenes/x.json") && !girl.message.contains("Thing.java"));

        let sound = &report.entries[1];

        assert_eq!(sound.count, 2);
        assert!(sound.message.contains("misc.flap"));
        assert_eq!(report.entries[2].source, "Strut");
    }

    #[test]
    fn the_real_log_of_a_run_reads() {
        let log = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../run/logs/latest.log");

        if let Ok(text) = std::fs::read_to_string(log) {
            let report = parse(&text);

            assert!(report.lines > 0);
            println!("{} entries, offer {:?}", report.entries.len(), report.offer);
        }
    }
}
