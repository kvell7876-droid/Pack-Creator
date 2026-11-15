//! What a small talk file may say. Small talk is what settlers say to each other, and offer the player, while they are on a break: files in `data/<namespace>/social/`, each a list of
//! entries with conditions (the game's `SocialRegistry`). The words a condition may use are fixed by the game, so they are listed here once for the checks, the editor's completion and
//! the form.

/// `line` is said between two settlers, `offer` to the player when a settler on break walks over.
pub const KINDS: [&str; 2] = ["line", "offer"];

/// How the chat is going.
pub const OUTCOMES: [&str; 4] = ["warm", "easy", "flat", "clash"];

pub const PERSONALITIES: [&str; 5] = ["shy", "bold", "motherly", "bratty", "stoic"];

/// The jobs a settler can hold.
pub const JOBS: [&str; 11] = ["farmer", "lumberjack", "fisherman", "guard", "hunter", "scout", "alchemist", "hauler", "mineworker", "blacksmith", "courtesan"];

pub const MOODS: [&str; 5] = ["happy", "content", "tired", "upset", "afraid"];

pub const TIMES: [&str; 2] = ["day", "night"];

pub const WEATHERS: [&str; 3] = ["clear", "rain", "thunder"];

/// Who in a pair may say a line: the one who started the chat, or the one who answers.
pub const ROLES: [&str; 2] = ["starter", "answerer"];

/// What the game fills in inside a text, written in braces.
pub const PLACEHOLDERS: [&str; 3] = ["name", "partner", "interest"];

/// Every key an entry may have.
pub const KEYS: [&str; 12] = ["id", "kind", "text", "outcome", "personality", "job", "mood", "time", "weather", "interest", "role", "weight"];

pub fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}
