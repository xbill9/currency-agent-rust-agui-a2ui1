//! Source repairs applied before parsing, ported from the reference SDK
//! (`a2ui.inference_formats.express.compiler`, a2ui-agent-sdk 0.11.0):
//! stripping `<a2ui>` sentinel tags, turning `--` comments into `//`, and
//! rejoining identifiers and data paths a model split across a line break.

use fancy_regex::{Captures, Regex};
use std::collections::HashSet;
use std::sync::LazyLock;

const IDENTIFIER: &str = r"[a-zA-Z_][a-zA-Z0-9_]*";
const IDENT_CHARS: &str = r"a-zA-Z0-9_";

static SQL_COMMENT_OR_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"([rR]"""[\s\S]*?"""|[rR]"[^\r\n"]*"|"""(?:\\[\s\S]|[^\\])*?"""|"(?:\\[\s\S]|[^\\])*?"|(?:#|//)[^\r\n]*|/\*[\s\S]*?\*/)|--([^\r\n]*)"#,
    )
    .unwrap()
});
static DECLARED_LHS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"(?m)^[ \t]*({IDENTIFIER})[ \t]*=(?!=)")).unwrap());
static SPLIT_LHS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(?m)^([ \t]*{IDENTIFIER})[ \t]*(\r?\n)[ \t]*([{IDENT_CHARS}]+)(?=[ \t]*=(?!=))"
    ))
    .unwrap()
});
static SPLIT_DATA_PATH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"(\$[{IDENT_CHARS}/]*/)[ \t]*(\r?\n)[ \t]*([{IDENT_CHARS}/]+)"
    ))
    .unwrap()
});
static IDENT_ONLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^{IDENTIFIER}$")).unwrap());
static ASSIGN_EQUALS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[ \t]*=(?!=)").unwrap());
static SPLIT_ARG_IDENT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"([(\[,:=][ \t]*\r?\n?[ \t]*|[(\[,:=][ \t]*)({IDENTIFIER})[ \t]*(\r?\n)[ \t]*([{IDENT_CHARS}]+)(?=[ \t]*[,)\]}}]|\s*\r?\n)"
    ))
    .unwrap()
});

static OPEN_SENTINEL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)<a2ui(?:\s[^>]*)?>").unwrap());
static CLOSE_SENTINEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)</a2ui\s*>").unwrap());
static SENTINEL_TOKEN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(concat!(
        r#"[rR]"""[\s\S]*?(?:"""|\z)|[rR]"[^\r\n"]*(?:"|\z)|"""(?:\\[\s\S]|[^\\])*?(?:"""|\z)|"(?:\\[\s\S]|[^\\"\r\n])*(?:"|\z)|(?:#|//|--)"#,
        r"[^\r\n]*|/\*[\s\S]*?(?:\*/|\z)"
    ))
    .unwrap()
});

pub fn open_sentinel() -> &'static Regex {
    &OPEN_SENTINEL
}
pub fn close_sentinel() -> &'static Regex {
    &CLOSE_SENTINEL
}
pub fn sentinel_token() -> &'static Regex {
    &SENTINEL_TOKEN
}

fn find_at<'t>(re: &Regex, s: &'t str, at: usize) -> Option<fancy_regex::Match<'t>> {
    re.find_from_pos(s, at).ok().flatten()
}

/// Strips outer `<a2ui>` / `</a2ui>` tags outside string literals and comments.
pub fn strip_sentinel_tags(source: &str) -> String {
    let mut segments: Vec<(bool, String)> = Vec::new();
    let mut pos = 0;
    for m in SENTINEL_TOKEN.find_iter(source).flatten() {
        if m.start() > pos {
            segments.push((false, source[pos..m.start()].to_string()));
        }
        let tok = m.as_str();
        let is_lit_or_block = tok.starts_with('"')
            || tok.starts_with('r')
            || tok.starts_with('R')
            || tok.starts_with("/*");
        if !is_lit_or_block {
            if let Some(cm) = find_at(&CLOSE_SENTINEL, tok, 0) {
                if cm.start() > 0 {
                    segments.push((true, tok[..cm.start()].to_string()));
                }
                segments.push((false, tok[cm.start()..].to_string()));
                pos = m.end();
                continue;
            }
        }
        segments.push((true, tok.to_string()));
        pos = m.end();
    }
    if pos < source.len() {
        segments.push((false, source[pos..].to_string()));
    }

    let has_open = segments
        .iter()
        .any(|(is_tok, seg)| !is_tok && find_at(&OPEN_SENTINEL, seg, 0).is_some());
    let mut out = String::new();
    let mut inside = !has_open;
    for (is_tok, seg) in &segments {
        if *is_tok {
            if inside {
                out.push_str(seg);
            }
            continue;
        }
        let mut cursor = 0;
        while cursor < seg.len() {
            if !inside {
                match find_at(&OPEN_SENTINEL, seg, cursor) {
                    None => break,
                    Some(om) => {
                        inside = true;
                        cursor = om.end();
                        continue;
                    }
                }
            }
            let close_m = find_at(&CLOSE_SENTINEL, seg, cursor);
            let open_m = find_at(&OPEN_SENTINEL, seg, cursor);
            if let Some(om) = &open_m {
                if close_m.as_ref().is_none_or(|cm| om.start() < cm.start()) {
                    out.push_str(&seg[cursor..om.start()]);
                    cursor = om.end();
                    continue;
                }
            }
            if let Some(cm) = close_m {
                out.push_str(&seg[cursor..cm.start()]);
                inside = false;
                cursor = cm.end();
                if !has_open {
                    return out;
                }
                continue;
            }
            out.push_str(&seg[cursor..]);
            break;
        }
    }
    out
}

/// Normalizes `--` comments and line-broken tokens outside strings and comments.
pub fn normalize_source(source: &str) -> String {
    let mut segments: Vec<(bool, String)> = Vec::new();
    let mut pos = 0;
    for caps in SQL_COMMENT_OR_TOKEN.captures_iter(source).flatten() {
        let m = caps.get(0).unwrap();
        if m.start() > pos {
            segments.push((false, source[pos..m.start()].to_string()));
        }
        match caps.get(1) {
            Some(tok) => segments.push((true, tok.as_str().to_string())),
            None => segments.push((
                true,
                format!("//{}", caps.get(2).map(|g| g.as_str()).unwrap_or("")),
            )),
        }
        pos = m.end();
    }
    if pos < source.len() {
        segments.push((false, source[pos..].to_string()));
    }

    // Bracket depth at every byte of each non-token segment.
    let mut depth = 0i32;
    let mut seg_depths: Vec<Vec<i32>> = Vec::new();
    for (is_tok, seg) in &segments {
        if *is_tok {
            seg_depths.push(Vec::new());
            continue;
        }
        let mut depths = vec![0; seg.len()];
        for (bi, ch) in seg.char_indices() {
            for d in depths.iter_mut().skip(bi).take(ch.len_utf8()) {
                *d = depth;
            }
            if "([{".contains(ch) {
                depth += 1;
            } else if ")]}".contains(ch) && depth > 0 {
                depth -= 1;
            }
        }
        seg_depths.push(depths);
    }

    let mut split_p2_starts: HashSet<(usize, usize)> = HashSet::new();
    for (si, (is_tok, seg)) in segments.iter().enumerate() {
        if *is_tok {
            continue;
        }
        for c in SPLIT_LHS.captures_iter(seg).flatten() {
            if seg_depths[si][c.get(1).unwrap().start()] == 0 {
                split_p2_starts.insert((si, c.get(3).unwrap().start()));
            }
        }
    }
    let mut direct_declared: HashSet<String> = HashSet::new();
    for (si, (is_tok, seg)) in segments.iter().enumerate() {
        if *is_tok {
            continue;
        }
        for c in DECLARED_LHS.captures_iter(seg).flatten() {
            let g = c.get(1).unwrap();
            if seg_depths[si][g.start()] == 0 && !split_p2_starts.contains(&(si, g.start())) {
                direct_declared.insert(g.as_str().to_string());
            }
        }
    }
    let mut declared = direct_declared.clone();
    for (si, (is_tok, seg)) in segments.iter().enumerate() {
        if *is_tok {
            continue;
        }
        for c in SPLIT_LHS.captures_iter(seg).flatten() {
            if seg_depths[si][c.get(1).unwrap().start()] == 0 {
                let p1 = c.get(1).unwrap().as_str().trim();
                let p2 = c.get(3).unwrap().as_str();
                if !direct_declared.contains(p1) && !direct_declared.contains(p2) {
                    declared.insert(format!("{p1}{p2}"));
                }
            }
        }
    }

    let mut out = String::new();
    for (is_tok, seg) in segments {
        if is_tok {
            out.push_str(&seg);
            continue;
        }
        let seg1 = replace_all(&SPLIT_DATA_PATH, &seg, |c| {
            let (p1, nl, p2) = (&c[1], &c[2], &c[3]);
            let rest = &seg[c.get(0).unwrap().end()..];
            if IDENT_ONLY.is_match(p2).unwrap_or(false)
                && ASSIGN_EQUALS.is_match(rest).unwrap_or(false)
            {
                c[0].to_string()
            } else {
                format!("{p1}{p2}{nl}")
            }
        });
        let seg2 = replace_all(&SPLIT_LHS, &seg1, |c| {
            let (p1_raw, nl, p2) = (&c[1], &c[2], &c[3]);
            let p1 = p1_raw.trim();
            if !direct_declared.contains(p1) && !direct_declared.contains(p2) {
                format!("{p1_raw}{p2}{nl}")
            } else {
                c[0].to_string()
            }
        });
        let seg3 = replace_all(&SPLIT_ARG_IDENT, &seg2, |c| {
            let (prefix, p1, nl, p2) = (&c[1], &c[2], &c[3], &c[4]);
            let joined = format!("{p1}{p2}");
            if declared.contains(&joined) && !declared.contains(p1) && !declared.contains(p2) {
                format!("{prefix}{joined}{nl}")
            } else {
                c[0].to_string()
            }
        });
        out.push_str(&seg3);
    }
    out
}

fn replace_all(re: &Regex, s: &str, mut f: impl FnMut(&Captures) -> String) -> String {
    let mut out = String::new();
    let mut last = 0;
    for c in re.captures_iter(s).flatten() {
        let m = c.get(0).unwrap();
        out.push_str(&s[last..m.start()]);
        out.push_str(&f(&c));
        last = m.end();
    }
    out.push_str(&s[last..]);
    out
}
