//! Finding Express `<a2ui>` blocks in a model response, and writing parts back.
//! A port of `BlockLexer` and `ExpressParser.unwrap/wrap/parse_response`
//! (a2ui-agent-sdk 0.11.0).

use super::ExpressError;
use super::compiler::Compiler;
use fancy_regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;

static LEADING_FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^```[a-zA-Z0-9_-]*\s*").unwrap());
static TRAILING_FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)\s*```[a-zA-Z0-9_-]*$").unwrap());
static OPEN_FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?m)^[ \t]*```[a-zA-Z0-9_-]*[ \t]*\r?\n?").unwrap());
static CLOSING_FENCE_END: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\r?\n?[ \t]*```[ \t]*\r?\n?$").unwrap());
static PAYLOAD_START: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^[ \t]*(?:surface\s*\(|deleteSurface\s*\(|\$[a-zA-Z0-9_/]*\s*=(?!=)|[a-zA-Z_][a-zA-Z0-9_]*\s*=(?!=))").unwrap()
});
static OPEN_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<a2ui(?:\s[^>]*)?>").unwrap());
static CLOSE_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</a2ui\s*>").unwrap());

const SINGLE_LINE_COMMENTS: [&str; 3] = ["#", "//", "--"];

/// One piece of a model response: prose, or a raw Express block.
#[derive(Debug, Clone, PartialEq)]
pub enum RawPart {
    Text(String),
    A2ui { raw: String, is_final: bool },
}

/// One piece of a parsed response: prose, or compiled A2UI messages.
#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Text(String),
    A2ui(Vec<Value>),
}

fn find_from(re: &Regex, s: &str, at: usize) -> Option<(usize, usize)> {
    re.find_from_pos(s, at).ok().flatten().map(|m| (m.start(), m.end()))
}

/// Matches `re` anchored at `i` within `s[..end]`.
fn match_at(re: &Regex, s: &str, i: usize, end: usize) -> Option<usize> {
    let hay = &s[..end];
    match re.find_from_pos(hay, i).ok().flatten() {
        Some(m) if m.start() == i => Some(m.end()),
        _ => None,
    }
}

fn sub_once(re: &Regex, s: &str) -> String {
    match re.find(s).ok().flatten() {
        Some(m) => format!("{}{}", &s[..m.start()], &s[m.end()..]),
        None => s.to_string(),
    }
}

fn clean_markdown(text: &str) -> String {
    let t = text.trim();
    let t = sub_once(&LEADING_FENCE, t);
    let t = sub_once(&TRAILING_FENCE, &t);
    let t = t.trim().to_string();
    if t.trim_matches('`').is_empty() { String::new() } else { t }
}

fn clean_raw(text: &str) -> String {
    clean_markdown(text).trim_matches('`').trim().to_string()
}

fn add_text(parts: &mut Vec<RawPart>, text: &str) {
    let c = clean_markdown(text);
    if !c.is_empty() {
        parts.push(RawPart::Text(c));
    }
}

fn next_char_len(s: &str, i: usize) -> usize {
    s[i..].chars().next().map(|c| c.len_utf8()).unwrap_or(1)
}

/// Scans `[start, end)` for the closing tag outside strings and block comments.
fn scan_block(content: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    #[derive(PartialEq)]
    enum St { Code, Str, Line, Block }
    let mut i = start;
    let mut st = St::Code;
    let mut triple = false;
    while i < end {
        match st {
            St::Code => {
                if let Some(e) = match_at(&CLOSE_TAG, content, i, end) {
                    return Some((i, e));
                }
                if content[i..].starts_with('"') {
                    if i + 2 < end && content[i..].starts_with("\"\"\"") {
                        triple = true;
                        i += 3;
                    } else {
                        triple = false;
                        i += 1;
                    }
                    st = St::Str;
                    continue;
                }
                if content[i..].starts_with("/*") {
                    i += 2;
                    st = St::Block;
                    continue;
                }
                if let Some(cm) = SINGLE_LINE_COMMENTS.iter().find(|cm| content[i..].starts_with(**cm)) {
                    i += cm.len();
                    st = St::Line;
                    continue;
                }
                i += next_char_len(content, i);
            }
            St::Str => {
                if content[i..].starts_with('\\') {
                    i += 1;
                    if i < content.len() {
                        i += next_char_len(content, i);
                    }
                    continue;
                }
                if triple {
                    if content[i..].starts_with("\"\"\"") {
                        i += 3;
                        st = St::Code;
                        continue;
                    }
                } else if content[i..].starts_with('"') {
                    i += 1;
                    st = St::Code;
                    continue;
                }
                i += next_char_len(content, i);
            }
            St::Line => {
                if let Some(e) = match_at(&CLOSE_TAG, content, i, end) {
                    return Some((i, e));
                }
                if content[i..].starts_with('\n') || content[i..].starts_with('\r') {
                    st = St::Code;
                }
                i += next_char_len(content, i);
            }
            St::Block => {
                if content[i..].starts_with("*/") {
                    i += 2;
                    st = St::Code;
                    continue;
                }
                i += next_char_len(content, i);
            }
        }
    }
    None
}

fn try_recover_unmatched_close(parts: &mut Vec<RawPart>, content: &str, start: usize, search_end: usize) -> Option<usize> {
    let seg = &content[start..search_end];
    find_from(&CLOSE_TAG, seg, 0)?;
    let fences: Vec<(usize, usize)> = OPEN_FENCE
        .find_iter(seg)
        .flatten()
        .map(|m| (m.start(), m.end()))
        .collect();
    let mut idx = 0;
    while idx < fences.len() {
        let (fs, fe) = fences[idx];
        idx += 2;
        let Some((cs, ce)) = scan_block(seg, fe, seg.len()) else { continue };
        let body = &seg[fe..cs];
        let stripped = sub_once(&CLOSING_FENCE_END, body);
        if find_from(&OPEN_FENCE, &stripped, 0).is_some() {
            continue;
        }
        let raw = clean_raw(&stripped);
        if raw.is_empty() {
            continue;
        }
        add_text(parts, &seg[..fs]);
        parts.push(RawPart::A2ui { raw, is_final: true });
        return Some(start + ce);
    }
    let mut search_from = 0;
    let mut pair = 0;
    while pair + 1 < fences.len() {
        if scan_block(seg, fences[pair + 1].1, seg.len()).is_none() {
            break;
        }
        search_from = fences[pair + 1].1;
        pair += 2;
    }
    let (ss, _) = find_from(&PAYLOAD_START, seg, search_from)?;
    let (cs, ce) = scan_block(seg, ss, seg.len())?;
    let raw = clean_raw(&seg[ss..cs]);
    if raw.is_empty() {
        return None;
    }
    add_text(parts, &seg[..ss]);
    parts.push(RawPart::A2ui { raw, is_final: true });
    Some(start + ce)
}

/// Splits a response into text parts and raw Express blocks.
pub fn unwrap(content: &str) -> Vec<RawPart> {
    let mut parts = Vec::new();
    let n = content.len();
    let mut i = 0;
    let mut recovered = false;
    while i < n {
        let open = find_from(&OPEN_TAG, content, i);
        let search_end = open.map(|(s, _)| s).unwrap_or(n);
        if !recovered && search_end > i {
            if let Some(next) = try_recover_unmatched_close(&mut parts, content, i, search_end) {
                recovered = true;
                i = next;
                continue;
            }
        }
        let Some((os, oe)) = open else { break };
        add_text(&mut parts, &content[i..os]);
        match scan_block(content, oe, n) {
            None => {
                parts.push(RawPart::A2ui { raw: clean_raw(&content[oe..]), is_final: false });
                return parts;
            }
            Some((cs, ce)) => {
                parts.push(RawPart::A2ui { raw: clean_raw(&content[oe..cs]), is_final: true });
                i = ce;
            }
        }
    }
    if i < n {
        add_text(&mut parts, &content[i..]);
    }
    parts
}

/// Writes parts back out the way a model would emit them.
pub fn wrap(parts: &[RawPart]) -> String {
    parts
        .iter()
        .map(|p| match p {
            RawPart::Text(t) => t.clone(),
            RawPart::A2ui { raw, .. } => format!("<a2ui>\n{raw}\n</a2ui>"),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether the response holds an Express block (a closed one when `complete`).
pub fn has_format_content(content: &str, complete: bool) -> bool {
    unwrap(content).iter().any(|p| match p {
        RawPart::A2ui { is_final, .. } => !complete || *is_final,
        _ => false,
    })
}

/// Unwraps a response and compiles each block it finds.
pub fn parse_response(compiler: &Compiler, content: &str, wrapped: bool, surface_id: &str) -> Result<Vec<Part>, ExpressError> {
    if !wrapped {
        return Ok(vec![Part::A2ui(compiler.compile(content, surface_id)?)]);
    }
    let mut out = Vec::new();
    for p in unwrap(content) {
        match p {
            RawPart::Text(t) => out.push(Part::Text(t)),
            RawPart::A2ui { raw, .. } => out.push(Part::A2ui(compiler.compile(&raw, surface_id)?)),
        }
    }
    Ok(out)
}
