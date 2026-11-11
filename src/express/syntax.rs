//! Lexer and recursive-descent parser for A2UI Express, following
//! `specification/inference_formats/express/Express.g4` (A2UI 1.0).

use super::ExpressError;
use serde_json::Number;

/// A parsed Express expression.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// `$...` data path, without the `$`.
    Path(String),
    /// A bare identifier.
    Var(String),
    /// The `_` placeholder.
    Skipped,
    Call(Call),
    Check(Check),
    Array(Vec<Node>),
    /// Map literal; keys keep their written order.
    Map(Vec<(String, Node)>),
    Str(String),
    Num(Number),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub name: String,
    pub args: Vec<Node>,
    /// Keyword arguments in written order. A repeated keyword keeps the last value.
    pub kwargs: Vec<(String, Node)>,
}

impl Call {
    pub fn kwarg(&self, key: &str) -> Option<&Node> {
        self.kwargs.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub fn has_kwarg(&self, key: &str) -> bool {
        self.kwargs.iter().any(|(k, _)| k == key)
    }
    pub fn remove_kwarg(&mut self, key: &str) -> Option<Node> {
        let idx = self.kwargs.iter().position(|(k, _)| k == key)?;
        Some(self.kwargs.remove(idx).1)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Check {
    pub name: String,
    pub args: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// `target = value`, where target is an identifier or a `$path` (kept with its `$`).
    Assign(String, Node),
    Expr(Node),
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    Comma,
    Colon,
    Eq,
    Underscore,
    Null,
    Bool(bool),
    Str(String),
    Num(Number),
    Path(String),
    Check(String),
    Ident(String),
    SysIdent(String),
    Eof,
}

fn is_ident_start(c: char) -> bool {
    c == '_' || unicode_ident::is_xid_start(c)
}

fn is_ident_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

/// Unescapes `\n`, `\r`, `\t`, `\\` and `\"`; any other escape stays as written.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match it.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some('"') => out.push('"'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// Parses a NUMBER token the way the reference SDK does: integer text becomes
/// an exact integer, anything else a finite double.
fn parse_number(text: &str) -> Result<Number, ExpressError> {
    let is_float = text.contains(['.', 'e', 'E']);
    if !is_float {
        let trimmed = text.trim_start_matches('+');
        if let Ok(i) = trimmed.parse::<i64>() {
            return Ok(Number::from(i));
        }
    }
    let v: f64 = text
        .trim_start_matches('+')
        .parse()
        .map_err(|_| ExpressError::Parse(format!("Invalid number literal {text}.")))?;
    if !v.is_finite() {
        return Err(ExpressError::Parse(format!(
            "Number literal {text} is out of range."
        )));
    }
    Number::from_f64(v).ok_or_else(|| ExpressError::Parse(format!("Invalid number {text}.")))
}

struct Lexer<'a> {
    src: &'a str,
    chars: Vec<(usize, char)>,
    i: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            chars: src.char_indices().collect(),
            i: 0,
        }
    }

    fn peek(&self, off: usize) -> Option<char> {
        self.chars.get(self.i + off).map(|(_, c)| *c)
    }

    fn byte_pos(&self, idx: usize) -> usize {
        self.chars.get(idx).map(|(b, _)| *b).unwrap_or(self.src.len())
    }

    fn rest(&self) -> &'a str {
        &self.src[self.byte_pos(self.i)..]
    }

    fn line_col(&self) -> (usize, usize) {
        let upto = &self.src[..self.byte_pos(self.i)];
        let line = upto.matches('\n').count() + 1;
        let col = upto.rsplit('\n').next().map(|l| l.chars().count()).unwrap_or(0);
        (line, col)
    }

    fn err(&self, msg: &str) -> ExpressError {
        let (l, c) = self.line_col();
        ExpressError::Parse(format!("Syntax error at line {l}:{c}: {msg}"))
    }

    fn advance_bytes(&mut self, n_bytes: usize) {
        let target = self.byte_pos(self.i) + n_bytes;
        while self.i < self.chars.len() && self.chars[self.i].0 < target {
            self.i += 1;
        }
    }

    fn skip_trivia(&mut self) {
        loop {
            match self.peek(0) {
                Some(c) if c == ' ' || c == '\t' || c == '\r' || c == '\n' || c == ';' => {
                    self.i += 1
                }
                Some('#') => self.skip_line(),
                Some('/') if self.peek(1) == Some('/') => self.skip_line(),
                Some('/') if self.peek(1) == Some('*') => {
                    let rest = self.rest();
                    match rest[2..].find("*/") {
                        Some(end) => self.advance_bytes(end + 4),
                        None => {
                            // Unterminated block comment: leave it for the error path.
                            return;
                        }
                    }
                }
                _ => return,
            }
        }
    }

    fn skip_line(&mut self) {
        while let Some(c) = self.peek(0) {
            if c == '\n' || c == '\r' {
                break;
            }
            self.i += 1;
        }
    }

    /// Length in bytes of a `"""...."""` body match starting at `start` (after the
    /// opening quotes), honoring backslash escapes. Returns the byte length up to
    /// and including the closing quotes.
    fn scan_escaped(rest: &str, open_len: usize, close: &str) -> Option<usize> {
        let bytes = rest.as_bytes();
        let mut j = open_len;
        while j < bytes.len() {
            if bytes[j] == b'\\' {
                // Skip the escaped character (which may be multi-byte).
                let next = rest[j + 1..].chars().next()?;
                j += 1 + next.len_utf8();
                continue;
            }
            if rest[j..].starts_with(close) {
                return Some(j + close.len());
            }
            j += rest[j..].chars().next()?.len_utf8();
        }
        None
    }

    fn next_tok(&mut self) -> Result<Tok, ExpressError> {
        self.skip_trivia();
        let Some(c) = self.peek(0) else {
            if self.rest().starts_with("/*") {
                return Err(self.err("unterminated block comment"));
            }
            return Ok(Tok::Eof);
        };
        if c == '/' && self.peek(1) == Some('*') {
            return Err(self.err("unterminated block comment"));
        }
        let simple = match c {
            '(' => Some(Tok::LParen),
            ')' => Some(Tok::RParen),
            '[' => Some(Tok::LBrack),
            ']' => Some(Tok::RBrack),
            '{' => Some(Tok::LBrace),
            '}' => Some(Tok::RBrace),
            ',' => Some(Tok::Comma),
            ':' => Some(Tok::Colon),
            '=' => Some(Tok::Eq),
            _ => None,
        };
        if let Some(t) = simple {
            self.i += 1;
            return Ok(t);
        }
        let rest = self.rest();

        // Raw strings: r"""...""" and r"..." (longest match against identifier `r`).
        if (c == 'r' || c == 'R') && self.peek(1) == Some('"') {
            if rest[1..].starts_with("\"\"\"") {
                if let Some(end) = rest[4..].find("\"\"\"") {
                    let body = rest[4..4 + end].to_string();
                    self.advance_bytes(4 + end + 3);
                    return Ok(Tok::Str(body));
                }
            }
            let body_end = rest[2..].find(['"', '\r', '\n']);
            if let Some(e) = body_end {
                if rest[2 + e..].starts_with('"') {
                    let body = rest[2..2 + e].to_string();
                    self.advance_bytes(2 + e + 1);
                    return Ok(Tok::Str(body));
                }
            }
            // Falls through to an identifier `r`.
        }
        if c == '"' {
            if rest.starts_with("\"\"\"") {
                if let Some(len) = Self::scan_escaped(rest, 3, "\"\"\"") {
                    let body = unescape(&rest[3..len - 3]);
                    self.advance_bytes(len);
                    return Ok(Tok::Str(body));
                }
            }
            if let Some(len) = Self::scan_escaped(rest, 1, "\"") {
                let inner = rest[1..len - 1].trim_matches(['\r', '\n']);
                let body = unescape(inner);
                self.advance_bytes(len);
                return Ok(Tok::Str(body));
            }
            return Err(self.err("unterminated string literal"));
        }
        if c == '$' {
            let n = rest[1..]
                .find(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '/'))
                .unwrap_or(rest.len() - 1);
            let p = rest[1..1 + n].to_string();
            self.advance_bytes(1 + n);
            return Ok(Tok::Path(p));
        }
        if (c == '?' || c == '@') && self.peek(1).is_some_and(is_ident_start) {
            self.i += 1;
            let name = self.ident_body();
            return Ok(if c == '?' {
                Tok::Check(name)
            } else {
                Tok::SysIdent(format!("@{name}"))
            });
        }
        if c.is_ascii_digit()
            || c == '.'
            || ((c == '+' || c == '-')
                && (self.peek(1).is_some_and(|d| d.is_ascii_digit())
                    || (self.peek(1) == Some('.')
                        && self.peek(2).is_some_and(|d| d.is_ascii_digit()))))
        {
            if let Some(len) = scan_number(rest) {
                let text = &rest[..len];
                let n = parse_number(text)?;
                self.advance_bytes(len);
                return Ok(Tok::Num(n));
            }
        }
        if is_ident_start(c) {
            let name = self.ident_body();
            return Ok(match name.as_str() {
                "_" => Tok::Underscore,
                "null" => Tok::Null,
                "true" => Tok::Bool(true),
                "false" => Tok::Bool(false),
                _ => Tok::Ident(name),
            });
        }
        Err(self.err(&format!("token recognition error at: '{c}'")))
    }

    fn ident_body(&mut self) -> String {
        let mut s = String::new();
        if let Some(c) = self.peek(0) {
            s.push(c);
            self.i += 1;
        }
        while let Some(c) = self.peek(0) {
            if !is_ident_continue(c) {
                break;
            }
            s.push(c);
            self.i += 1;
        }
        s
    }
}

/// Matches `[+-]? ([0-9]+ ('.' [0-9]*)? | '.' [0-9]+) ([eE] [+-]? [0-9]+)?`.
fn scan_number(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut j = 0;
    if j < b.len() && (b[j] == b'+' || b[j] == b'-') {
        j += 1;
    }
    let int_start = j;
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
    }
    if j > int_start {
        if j < b.len() && b[j] == b'.' {
            j += 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
        }
    } else {
        if j < b.len() && b[j] == b'.' {
            j += 1;
        } else {
            return None;
        }
        let frac = j;
        while j < b.len() && b[j].is_ascii_digit() {
            j += 1;
        }
        if j == frac {
            return None;
        }
    }
    if j < b.len() && (b[j] == b'e' || b[j] == b'E') {
        let mut k = j + 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        let exp = k;
        while k < b.len() && b[k].is_ascii_digit() {
            k += 1;
        }
        if k > exp {
            j = k;
        }
    }
    Some(j)
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos]
    }
    fn peek2(&self) -> &Tok {
        self.toks.get(self.pos + 1).unwrap_or(&Tok::Eof)
    }
    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn expect(&mut self, t: Tok, what: &str) -> Result<(), ExpressError> {
        if *self.peek() == t {
            self.bump();
            Ok(())
        } else {
            Err(ExpressError::Parse(format!(
                "Syntax error: expected {what}, found {:?}",
                self.peek()
            )))
        }
    }

    fn program(&mut self) -> Result<Vec<Stmt>, ExpressError> {
        let mut out = Vec::new();
        while *self.peek() != Tok::Eof {
            out.push(self.statement()?);
        }
        Ok(out)
    }

    fn statement(&mut self) -> Result<Stmt, ExpressError> {
        match (self.peek().clone(), self.peek2()) {
            (Tok::Ident(name), Tok::Eq) => {
                self.bump();
                self.bump();
                Ok(Stmt::Assign(name, self.expression()?))
            }
            (Tok::Path(p), Tok::Eq) => {
                self.bump();
                self.bump();
                Ok(Stmt::Assign(format!("${p}"), self.expression()?))
            }
            _ => Ok(Stmt::Expr(self.expression()?)),
        }
    }

    fn expression(&mut self) -> Result<Node, ExpressError> {
        match self.bump() {
            Tok::LBrack => {
                let items = self.list(Tok::RBrack, |p| p.expression())?;
                Ok(Node::Array(items))
            }
            Tok::LBrace => {
                let entries = self.list(Tok::RBrace, |p| p.map_entry())?;
                // A repeated key keeps the last value, in its first position.
                let mut out: Vec<(String, Node)> = Vec::new();
                for (k, v) in entries {
                    if let Some(slot) = out.iter_mut().find(|(ek, _)| *ek == k) {
                        slot.1 = v;
                    } else {
                        out.push((k, v));
                    }
                }
                Ok(Node::Map(out))
            }
            Tok::Path(p) => Ok(Node::Path(p)),
            Tok::Check(name) => {
                let mut args = Vec::new();
                if *self.peek() == Tok::LParen {
                    self.bump();
                    args = self.list(Tok::RParen, |p| p.expression())?;
                }
                Ok(Node::Check(Check { name, args }))
            }
            Tok::Ident(name) | Tok::SysIdent(name) if *self.peek() == Tok::LParen => {
                self.bump();
                let raw = self.list(Tok::RParen, |p| p.arg())?;
                let mut call = Call {
                    name,
                    args: Vec::new(),
                    kwargs: Vec::new(),
                };
                for (k, v) in raw {
                    match k {
                        None => call.args.push(v),
                        Some(k) => {
                            if let Some(slot) = call.kwargs.iter_mut().find(|(ek, _)| *ek == k) {
                                slot.1 = v;
                            } else {
                                call.kwargs.push((k, v));
                            }
                        }
                    }
                }
                Ok(Node::Call(call))
            }
            Tok::Ident(name) => Ok(Node::Var(name)),
            Tok::Underscore => Ok(Node::Skipped),
            Tok::Str(s) => Ok(Node::Str(s)),
            Tok::Num(n) => Ok(Node::Num(n)),
            Tok::Bool(b) => Ok(Node::Bool(b)),
            Tok::Null => Ok(Node::Null),
            other => Err(ExpressError::Parse(format!(
                "Syntax error: unexpected {other:?}"
            ))),
        }
    }

    fn map_entry(&mut self) -> Result<(String, Node), ExpressError> {
        let key = match self.bump() {
            Tok::Ident(k) => k,
            Tok::Str(k) => k,
            other => {
                return Err(ExpressError::Parse(format!(
                    "Syntax error: expected a map key, found {other:?}"
                )));
            }
        };
        self.expect(Tok::Colon, "':'")?;
        Ok((key, self.expression()?))
    }

    fn arg(&mut self) -> Result<(Option<String>, Node), ExpressError> {
        if let (Tok::Ident(name), Tok::Eq) = (self.peek().clone(), self.peek2()) {
            self.bump();
            self.bump();
            return Ok((Some(name), self.expression()?));
        }
        Ok((None, self.expression()?))
    }

    /// Parses `item (',' item)* ','? close`, with the opener already consumed.
    fn list<T>(
        &mut self,
        close: Tok,
        mut item: impl FnMut(&mut Self) -> Result<T, ExpressError>,
    ) -> Result<Vec<T>, ExpressError> {
        let mut out = Vec::new();
        if *self.peek() == close {
            self.bump();
            return Ok(out);
        }
        loop {
            out.push(item(self)?);
            match self.peek() {
                Tok::Comma => {
                    self.bump();
                    if *self.peek() == close {
                        self.bump();
                        return Ok(out);
                    }
                }
                t if *t == close => {
                    self.bump();
                    return Ok(out);
                }
                other => {
                    return Err(ExpressError::Parse(format!(
                        "Syntax error: expected ',' or {close:?}, found {other:?}"
                    )));
                }
            }
        }
    }
}

/// Parses an Express program (already stripped of sentinels and normalized).
pub fn parse_program(src: &str) -> Result<Vec<Stmt>, ExpressError> {
    let mut lx = Lexer::new(src);
    let mut toks = Vec::new();
    loop {
        let t = lx.next_tok()?;
        let end = t == Tok::Eof;
        toks.push(t);
        if end {
            break;
        }
    }
    Parser { toks, pos: 0 }.program()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_assignment_and_call() {
        let s = parse_program(r#"root = Text("Hi", variant="caption")"#).unwrap();
        assert_eq!(s.len(), 1);
        let Stmt::Assign(name, Node::Call(c)) = &s[0] else { panic!() };
        assert_eq!(name, "root");
        assert_eq!(c.name, "Text");
        assert_eq!(c.kwargs[0].0, "variant");
    }

    #[test]
    fn numbers_match_json_forms() {
        for (t, ok) in [(".5", true), ("1.", true), ("+3", true), ("1e-7", true), ("1e999", false)] {
            assert_eq!(parse_program(&format!("x = {t}")).is_ok(), ok, "{t}");
        }
    }
}
