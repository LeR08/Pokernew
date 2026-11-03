//! The stat expression language.
//!
//! ```text
//! expr    := or
//! or      := and ("or" and)*
//! and     := not ("and" not)*
//! not     := "not" not | cmp
//! cmp     := sum (("=" | "!=" | "<" | "<=" | ">" | ">=") sum)?
//! sum     := product (("+" | "-") product)*
//! product := unary (("*" | "/") unary)*
//! unary   := "-" unary | primary
//! primary := number | 'string' | ident | ident "(" args ")" | "(" expr ")"
//! ```
//!
//! Identifiers are counters (`cnt_vpip`, `opp_3bet`…) or hand fields
//! (`position`, `game_type`…); the only function is `if(cond, a, b)`.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(f64),
    Str(String),
    Ident(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl BinOp {
    pub fn sql(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Eq => "=",
            BinOp::Ne => "<>",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "AND",
            BinOp::Or => "OR",
        }
    }
}

/// A syntax error, with the character position for the editor.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize)]
#[error("{message} (position {pos})")]
pub struct SyntaxError {
    pub pos: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Op(&'static str),
    LParen,
    RParen,
    Comma,
    End,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Num(n) => write!(f, "{n}"),
            Tok::Str(s) => write!(f, "'{s}'"),
            Tok::Ident(s) => write!(f, "{s}"),
            Tok::Op(o) => write!(f, "{o}"),
            Tok::LParen => write!(f, "("),
            Tok::RParen => write!(f, ")"),
            Tok::Comma => write!(f, ","),
            Tok::End => write!(f, "end of formula"),
        }
    }
}

fn lex(src: &str) -> Result<Vec<(usize, Tok)>, SyntaxError> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i] as char;
        let start = i;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let tok =
            if c.is_ascii_digit() || (c == '.' && b.get(i + 1).is_some_and(u8::is_ascii_digit)) {
                while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                    i += 1;
                }
                let s = &src[start..i];
                Tok::Num(s.parse().map_err(|_| SyntaxError {
                    pos: start,
                    message: format!("bad number '{s}'"),
                })?)
            } else if c.is_ascii_alphabetic() || c == '_' {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                Tok::Ident(src[start..i].to_ascii_lowercase())
            } else if c == '\'' {
                i += 1;
                let s0 = i;
                while i < b.len() && b[i] != b'\'' {
                    i += 1;
                }
                if i >= b.len() {
                    return Err(SyntaxError {
                        pos: start,
                        message: "unterminated string".into(),
                    });
                }
                let s = src[s0..i].to_string();
                i += 1;
                Tok::Str(s)
            } else {
                let two = src.get(i..i + 2).unwrap_or("");
                let op: Option<&'static str> = match two {
                    "<=" => Some("<="),
                    ">=" => Some(">="),
                    "!=" => Some("!="),
                    "<>" => Some("!="),
                    "==" => Some("="),
                    _ => None,
                };
                if let Some(op) = op {
                    i += 2;
                    Tok::Op(op)
                } else {
                    i += 1;
                    match c {
                        '+' => Tok::Op("+"),
                        '-' => Tok::Op("-"),
                        '*' => Tok::Op("*"),
                        '/' => Tok::Op("/"),
                        '=' => Tok::Op("="),
                        '<' => Tok::Op("<"),
                        '>' => Tok::Op(">"),
                        '(' => Tok::LParen,
                        ')' => Tok::RParen,
                        ',' => Tok::Comma,
                        _ => {
                            return Err(SyntaxError {
                                pos: start,
                                message: format!("unexpected character '{c}'"),
                            })
                        }
                    }
                }
            };
        out.push((start, tok));
    }
    out.push((src.len(), Tok::End));
    Ok(out)
}

struct Parser {
    toks: Vec<(usize, Tok)>,
    i: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.i].1
    }
    fn pos(&self) -> usize {
        self.toks[self.i].0
    }
    fn next(&mut self) -> Tok {
        let t = self.toks[self.i].1.clone();
        if self.i + 1 < self.toks.len() {
            self.i += 1;
        }
        t
    }
    fn err<T>(&self, message: impl Into<String>) -> Result<T, SyntaxError> {
        Err(SyntaxError {
            pos: self.pos(),
            message: message.into(),
        })
    }
    fn is_word(&self, w: &str) -> bool {
        matches!(self.peek(), Tok::Ident(s) if s == w)
    }

    fn or(&mut self) -> Result<Expr, SyntaxError> {
        let mut l = self.and()?;
        while self.is_word("or") {
            self.next();
            let r = self.and()?;
            l = Expr::Bin(BinOp::Or, Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn and(&mut self) -> Result<Expr, SyntaxError> {
        let mut l = self.not()?;
        while self.is_word("and") {
            self.next();
            let r = self.not()?;
            l = Expr::Bin(BinOp::And, Box::new(l), Box::new(r));
        }
        Ok(l)
    }
    fn not(&mut self) -> Result<Expr, SyntaxError> {
        if self.is_word("not") {
            self.next();
            return Ok(Expr::Not(Box::new(self.not()?)));
        }
        self.cmp()
    }
    fn cmp(&mut self) -> Result<Expr, SyntaxError> {
        let l = self.sum()?;
        let op = match self.peek() {
            Tok::Op("=") => BinOp::Eq,
            Tok::Op("!=") => BinOp::Ne,
            Tok::Op("<") => BinOp::Lt,
            Tok::Op("<=") => BinOp::Le,
            Tok::Op(">") => BinOp::Gt,
            Tok::Op(">=") => BinOp::Ge,
            _ => return Ok(l),
        };
        self.next();
        let r = self.sum()?;
        Ok(Expr::Bin(op, Box::new(l), Box::new(r)))
    }
    fn sum(&mut self) -> Result<Expr, SyntaxError> {
        let mut l = self.product()?;
        loop {
            let op = match self.peek() {
                Tok::Op("+") => BinOp::Add,
                Tok::Op("-") => BinOp::Sub,
                _ => return Ok(l),
            };
            self.next();
            let r = self.product()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }
    fn product(&mut self) -> Result<Expr, SyntaxError> {
        let mut l = self.unary()?;
        loop {
            let op = match self.peek() {
                Tok::Op("*") => BinOp::Mul,
                Tok::Op("/") => BinOp::Div,
                _ => return Ok(l),
            };
            self.next();
            let r = self.unary()?;
            l = Expr::Bin(op, Box::new(l), Box::new(r));
        }
    }
    fn unary(&mut self) -> Result<Expr, SyntaxError> {
        if matches!(self.peek(), Tok::Op("-")) {
            self.next();
            return Ok(Expr::Neg(Box::new(self.unary()?)));
        }
        self.primary()
    }
    fn primary(&mut self) -> Result<Expr, SyntaxError> {
        let at = self.i;
        match self.next() {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Ident(name) if ["and", "or", "not"].contains(&name.as_str()) => {
                self.i = at;
                self.err(format!("unexpected '{name}'"))
            }
            Tok::Ident(name) => {
                if matches!(self.peek(), Tok::LParen) {
                    self.next();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), Tok::RParen) {
                        loop {
                            args.push(self.or()?);
                            if matches!(self.peek(), Tok::Comma) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    if !matches!(self.next(), Tok::RParen) {
                        return self.err("expected ')'");
                    }
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Tok::LParen => {
                let e = self.or()?;
                match self.next() {
                    Tok::RParen => Ok(e),
                    _ => self.err("expected ')'"),
                }
            }
            t => {
                self.i = at;
                self.err(format!("unexpected {t}"))
            }
        }
    }
}

/// Parses a formula or a condition.
pub fn parse(src: &str) -> Result<Expr, SyntaxError> {
    if src.trim().is_empty() {
        return Err(SyntaxError {
            pos: 0,
            message: "empty formula".into(),
        });
    }
    let mut p = Parser {
        toks: lex(src)?,
        i: 0,
    };
    let e = p.or()?;
    if !matches!(p.peek(), Tok::End) {
        let t = p.peek().clone();
        return p.err(format!("unexpected {t}"));
    }
    Ok(e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precedence() {
        let e = parse("a + b * 2 > 3 and not c = 'BB' or d").unwrap();
        let Expr::Bin(BinOp::Or, l, _) = e else {
            panic!()
        };
        let Expr::Bin(BinOp::And, cmp, not) = *l else {
            panic!()
        };
        assert!(matches!(*cmp, Expr::Bin(BinOp::Gt, _, _)));
        assert!(matches!(*not, Expr::Not(_)));
    }

    #[test]
    fn calls_and_errors() {
        assert!(
            matches!(parse("if(x > 1, 2, 3)").unwrap(), Expr::Call(n, a) if n == "if" && a.len() == 3)
        );
        let e = parse("cnt_vpip / ").unwrap_err();
        assert_eq!(e.pos, 11);
        assert!(parse("(a").is_err());
        assert!(parse("a b").is_err());
        assert!(parse("'open").is_err());
        assert!(parse("").is_err());
        assert!(parse("a $ b").unwrap_err().message.contains('$'));
    }
}
