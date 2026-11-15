//! The few JavaScript behaviours the converter's output depends on, so the Rust output is the same bytes: the order `Object.entries` gives, and what `Number("...")` accepts.

use serde_json::{Map, Value};

/// The entries of a JSON object in the order JavaScript lists them: keys that are array indexes (`"0"`, `"12"`) first, in numeric order, then the rest as written.
pub fn entries(map: &Map<String, Value>) -> Vec<(&String, &Value)> {
    let index = |key: &str| -> Option<u64> {
        let canonical = key == "0" || (!key.starts_with('0') && !key.is_empty() && key.bytes().all(|b| b.is_ascii_digit()));

        canonical.then(|| key.parse::<u64>().ok()).flatten().filter(|n| *n < 4_294_967_295)
    };
    let mut numeric: Vec<(u64, (&String, &Value))> = Vec::new();
    let mut rest = Vec::new();

    for (key, value) in map {
        match index(key) {
            Some(n) => numeric.push((n, (key, value))),
            None => rest.push((key, value)),
        }
    }

    numeric.sort_by_key(|(n, _)| *n);
    numeric.into_iter().map(|(_, entry)| entry).chain(rest).collect()
}

/// `Number(text)`: NaN when it is not a number. Accepts what JavaScript does (surrounding space, `.5`, `5.`, `1e3`, `0x1f`, `0b11`, `0o7`, `Infinity`) and nothing it does not.
pub fn number(text: &str) -> f64 {
    let t = text.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');

    if t.is_empty() {
        return 0.0;
    }

    for (prefix, radix) in [("0x", 16), ("0X", 16), ("0b", 2), ("0B", 2), ("0o", 8), ("0O", 8)] {
        if let Some(digits) = t.strip_prefix(prefix) {
            return if digits.is_empty() { f64::NAN } else { u64::from_str_radix(digits, radix).map(|n| n as f64).unwrap_or(f64::NAN) };
        }
    }

    let unsigned = t.strip_prefix(['+', '-']).unwrap_or(t);

    if unsigned == "Infinity" {
        return if t.starts_with('-') { f64::NEG_INFINITY } else { f64::INFINITY };
    }

    if !unsigned.starts_with(|c: char| c.is_ascii_digit() || c == '.') || unsigned.contains(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))) {
        return f64::NAN;
    }

    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// `Number.isFinite(Number(text))`.
pub fn finite(text: &str) -> bool {
    number(text).is_finite()
}

/// A number as JSON gave it, or None when the value is not a number.
pub fn as_number(value: &Value) -> Option<f64> {
    value.as_f64().filter(|n| n.is_finite())
}

/// `typeof value === 'number' && Number.isFinite(value) ? value : fallback`.
pub fn number_or(value: Option<&Value>, fallback: f64) -> f64 {
    value.and_then(as_number).unwrap_or(fallback)
}

/// `value === 0 ? 0 : -value`: never a negative zero.
pub fn negated(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { -value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn lists_entries_the_way_javascript_does() {
        let value = json!({ "b": 1, "10": 2, "a": 3, "2": 4, "0.5": 5, "01": 6 });
        let keys: Vec<&str> = entries(value.as_object().unwrap()).iter().map(|(k, _)| k.as_str()).collect();

        assert_eq!(keys, vec!["2", "10", "b", "a", "0.5", "01"]);
    }

    #[test]
    fn reads_numbers_the_way_javascript_does() {
        assert_eq!(number(" 12.5 "), 12.5);
        assert_eq!(number(".5"), 0.5);
        assert_eq!(number("5."), 5.0);
        assert_eq!(number("-1e2"), -100.0);
        assert_eq!(number("0x1f"), 31.0);
        assert_eq!(number(""), 0.0);
        assert!(number("abc").is_nan());
        assert!(number("1_0").is_nan());
        assert!(number("nan").is_nan());
        assert!(number("inf").is_nan());
        assert!(!finite("Infinity"));
        assert!(finite("7"));
        assert!(number("1 2").is_nan());
    }
}
