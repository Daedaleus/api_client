//! Bruno's `.bru` text format, as far as an import needs it: a file is a
//! list of blocks.
//!
//! ```text
//! meta {
//!   name: Get users
//!   seq: 1
//! }
//!
//! body:json {
//!   {
//!     "a": 1
//!   }
//! }
//! ```
//!
//! A block starts with `name {` (or `name [` for lists) at the start of a
//! line and ends with `}` (or `]`) alone at the start of a line; its lines
//! are indented by two spaces. Dictionary blocks hold `key: value` lines, a
//! leading `~` marks a disabled entry; text blocks (bodies, scripts, docs)
//! hold text as it is.

/// One block of a `.bru` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Block {
    /// E.g. `meta`, `get`, `body:json`, `auth:bearer`.
    pub name: String,
    /// The lines inside, without the two spaces of indentation.
    pub lines: Vec<String>,
}

/// An entry of a dictionary block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

/// The blocks of a `.bru` file; unclosed blocks run to the end.
pub(super) fn blocks(text: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut current: Option<(Block, char)> = None;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some((block, close)) = &mut current {
            if line.len() == 1 && line.starts_with(*close) {
                let (block, _) = current.take().unwrap_or_else(|| unreachable!());
                blocks.push(block);
            } else {
                block
                    .lines
                    .push(line.strip_prefix("  ").unwrap_or(line).to_owned());
            }
            continue;
        }
        let start = line
            .strip_suffix(" {")
            .map(|name| (name, '}'))
            .or_else(|| line.strip_suffix(" [").map(|name| (name, ']')));
        if let Some((name, close)) = start
            && !name.is_empty()
            && !name.starts_with(' ')
        {
            current = Some((
                Block {
                    name: name.to_owned(),
                    lines: Vec::new(),
                },
                close,
            ));
        }
    }
    if let Some((block, _)) = current {
        blocks.push(block);
    }
    blocks
}

impl Block {
    /// The `key: value` entries of a dictionary block.
    pub(super) fn entries(&self) -> Vec<Entry> {
        self.lines
            .iter()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let (enabled, line) = match line.strip_prefix('~') {
                    Some(rest) => (false, rest),
                    None => (true, line.as_str()),
                };
                let (key, value) = line.split_once(':').unwrap_or((line, ""));
                Entry {
                    key: key.trim().to_owned(),
                    value: value.trim().to_owned(),
                    enabled,
                }
            })
            .collect()
    }

    /// The value of `key` in a dictionary block.
    pub(super) fn get(&self, key: &str) -> Option<String> {
        self.entries()
            .into_iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.value)
    }

    /// The lines of a list block (`vars:secret [ … ]`), one name each.
    pub(super) fn items(&self) -> Vec<String> {
        self.lines
            .iter()
            .map(|line| line.trim().trim_end_matches(',').to_owned())
            .filter(|line| !line.is_empty())
            .collect()
    }

    /// The text of a text block.
    pub(super) fn text(&self) -> String {
        self.lines.join("\n")
    }
}

/// The first block named `name`.
pub(super) fn find<'a>(blocks: &'a [Block], name: &str) -> Option<&'a Block> {
    blocks.iter().find(|block| block.name == name)
}
