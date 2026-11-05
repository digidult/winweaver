//! The small expressions of a description: integers, the paths of the fields read so far
//! (`nt_headers.file_header.size_of_optional_header`, `optional_header.@offset`), reads of the file
//! (`u16(@here)`), and the operators of Rust in its precedence: `* / %`, `+ -`, `<< >>`, `&`, `^`,
//! `|`, the comparisons, `&&`, `||`, with `-` and `!` before a value. Every value is an `i128`,
//! wide enough for the 64-bit fields of a file in both signs. The messages of a wrong
//! expression are for the author of a description and stay English; the ones of a value read
//! show in the tree and are translated.

use wwrt::ww::i18n::{tr, trf};

/// A part of a path after its first name.
#[derive(Clone, Debug, PartialEq)]
pub enum Part {
    Name(String),
    /// The offset in the file of the node the path names so far.
    Offset,
    /// Its size in bytes.
    Size,
    /// Its offset and size together: the byte after it.
    End,
    /// The items of an array.
    Count,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(i128),
    /// A path from the scope of the field being read: `@here`, the place the next field would be
    /// read at, when the path is empty and `here` is set.
    Path {
        here: bool,
        parts: Vec<Part>,
    },
    /// A read of an unsigned integer of 1, 2, 4 or 8 bytes at an offset of the file.
    Read(u8, Box<Expr>),
    Not(Box<Expr>),
    Negative(Box<Expr>),
    Binary(Op, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Multiply,
    Divide,
    Remainder,
    Add,
    Subtract,
    Left,
    Right,
    And,
    Xor,
    Or,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Both,
    Either,
}

/// What an expression asks of the reader of a file.
pub trait Scope {
    /// The value of a path (a number, or `@offset`, `@size`, `@end`, `@count` of a node).
    fn path(&self, parts: &[Part]) -> Result<i128, String>;
    /// The place the next field of the structure being read would be read at.
    fn here(&self) -> i128;
    /// An unsigned integer of `width` bytes at `at` in the file.
    fn read(&mut self, width: u8, at: u64) -> Result<i128, String>;
}

/// The longest expression a description may hold, in characters.
const LONGEST: usize = 1000;
/// How deep the brackets and the operators of one expression may go.
const DEEPEST: usize = 256;

/// The tokens of an expression.
#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(i128),
    Name(String),
    At(String),
    Symbol(&'static str),
}

fn tokens(text: &str) -> Result<Vec<Token>, String> {
    if text.len() > LONGEST {
        return Err(format!("an expression longer than {LONGEST} characters"));
    }
    const SYMBOLS: [&str; 22] = [
        "<<", ">>", "<=", ">=", "==", "!=", "&&", "||", "+", "-", "*", "/", "%", "&", "|", "^",
        "<", ">", "!", "(", ")", ".",
    ];
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().filter(|c| **c != '_').collect();
            let value = if let Some(hex) = word.strip_prefix("0x").or(word.strip_prefix("0X")) {
                i128::from_str_radix(hex, 16)
            } else {
                word.parse::<i128>()
            };
            out.push(Token::Number(
                value.map_err(|_| format!("not a number: {word}"))?,
            ));
        } else if c.is_ascii_alphabetic() || c == '_' || c == '@' {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            out.push(match word.strip_prefix('@') {
                Some(at) => Token::At(at.to_owned()),
                None => Token::Name(word),
            });
        } else {
            let rest: String = chars[i..chars.len().min(i + 2)].iter().collect();
            let Some(symbol) = SYMBOLS.iter().find(|s| rest.starts_with(**s)) else {
                return Err(format!("an unknown sign: {c}"));
            };
            out.push(Token::Symbol(symbol));
            i += symbol.len();
        }
    }
    Ok(out)
}

/// The operators of a level of precedence, the weakest first.
const LEVELS: [&[(&str, Op)]; 9] = [
    &[("||", Op::Either)],
    &[("&&", Op::Both)],
    &[
        ("==", Op::Equal),
        ("!=", Op::NotEqual),
        ("<=", Op::LessEqual),
        (">=", Op::GreaterEqual),
        ("<", Op::Less),
        (">", Op::Greater),
    ],
    &[("|", Op::Or)],
    &[("^", Op::Xor)],
    &[("&", Op::And)],
    &[("<<", Op::Left), (">>", Op::Right)],
    &[("+", Op::Add), ("-", Op::Subtract)],
    &[("*", Op::Multiply), ("/", Op::Divide), ("%", Op::Remainder)],
];

struct Parser {
    tokens: Vec<Token>,
    at: usize,
    depth: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    fn symbol(&mut self, s: &'static str) -> bool {
        if self.peek() == Some(&Token::Symbol(s)) {
            self.at += 1;
            true
        } else {
            false
        }
    }

    fn deeper(&mut self) -> Result<(), String> {
        self.depth += 1;
        if self.depth > DEEPEST {
            return Err(format!("an expression deeper than {DEEPEST}"));
        }
        Ok(())
    }

    fn level(&mut self, n: usize) -> Result<Expr, String> {
        if n == LEVELS.len() {
            return self.unary();
        }
        self.deeper()?;
        let mut left = self.level(n + 1)?;
        'more: loop {
            for (s, op) in LEVELS[n] {
                if self.symbol(s) {
                    let right = self.level(n + 1)?;
                    left = Expr::Binary(*op, Box::new(left), Box::new(right));
                    continue 'more;
                }
            }
            break;
        }
        self.depth -= 1;
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        self.deeper()?;
        let expr = if self.symbol("-") {
            Expr::Negative(Box::new(self.unary()?))
        } else if self.symbol("!") {
            Expr::Not(Box::new(self.unary()?))
        } else {
            self.primary()?
        };
        self.depth -= 1;
        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.peek().cloned() {
            Some(Token::Number(n)) => {
                self.at += 1;
                Ok(Expr::Number(n))
            }
            Some(Token::Symbol("(")) => {
                self.at += 1;
                let inner = self.level(0)?;
                if !self.symbol(")") {
                    return Err("a bracket that does not close".to_owned());
                }
                Ok(inner)
            }
            Some(Token::Name(name)) => {
                self.at += 1;
                let width = match name.as_str() {
                    "u8" => Some(1),
                    "u16" => Some(2),
                    "u32" => Some(4),
                    "u64" => Some(8),
                    _ => None,
                };
                if let Some(width) = width
                    && self.symbol("(")
                {
                    let at = self.level(0)?;
                    if !self.symbol(")") {
                        return Err(format!("{name}( does not close"));
                    }
                    return Ok(Expr::Read(width, Box::new(at)));
                }
                self.path(false, vec![Part::Name(name)])
            }
            Some(Token::At(name)) => {
                self.at += 1;
                match name.as_str() {
                    "here" => self.path(true, Vec::new()),
                    _ => self.path(false, vec![at_part(&name)?]),
                }
            }
            Some(t) => Err(format!("an expression cannot go on with {t:?}")),
            None => Err("an expression ends too soon".to_owned()),
        }
    }

    fn path(&mut self, here: bool, mut parts: Vec<Part>) -> Result<Expr, String> {
        while self.symbol(".") {
            match self.peek().cloned() {
                Some(Token::Name(n)) => parts.push(Part::Name(n)),
                Some(Token::At(n)) => parts.push(at_part(&n)?),
                _ => return Err("a path ends with a dot".to_owned()),
            }
            self.at += 1;
        }
        Ok(Expr::Path { here, parts })
    }
}

fn at_part(name: &str) -> Result<Part, String> {
    match name {
        "offset" => Ok(Part::Offset),
        "size" => Ok(Part::Size),
        "end" => Ok(Part::End),
        "count" => Ok(Part::Count),
        _ => Err(format!("an unknown name: @{name}")),
    }
}

/// An expression of a description.
pub fn parse(text: &str) -> Result<Expr, String> {
    let mut parser = Parser {
        tokens: tokens(text)?,
        at: 0,
        depth: 0,
    };
    let expr = parser.level(0)?;
    if parser.at != parser.tokens.len() {
        return Err(format!("more after the expression in {text:?}"));
    }
    Ok(expr)
}

/// The value of an expression.
pub fn eval(expr: &Expr, scope: &mut dyn Scope) -> Result<i128, String> {
    Ok(match expr {
        Expr::Number(n) => *n,
        Expr::Path { here: true, parts } if parts.is_empty() => scope.here(),
        Expr::Path { parts, .. } => scope.path(parts)?,
        Expr::Read(width, at) => {
            let at = eval(at, scope)?;
            let at = u64::try_from(at).map_err(|_| trf("a read at {at}", &[("at", &at)]))?;
            scope.read(*width, at)?
        }
        Expr::Not(e) => i128::from(eval(e, scope)? == 0),
        Expr::Negative(e) => eval(e, scope)?.wrapping_neg(),
        Expr::Binary(op, a, b) => {
            let a = eval(a, scope)?;
            // `&&` and `||` read their right side only when it decides.
            match op {
                Op::Both if a == 0 => return Ok(0),
                Op::Either if a != 0 => return Ok(1),
                _ => {}
            }
            let b = eval(b, scope)?;
            let shift = |b: i128| u32::try_from(b).ok().filter(|s| *s < 128);
            match op {
                Op::Multiply => a.wrapping_mul(b),
                Op::Divide => a.checked_div(b).ok_or(tr("a division by zero"))?,
                Op::Remainder => a.checked_rem(b).ok_or(tr("a division by zero"))?,
                Op::Add => a.wrapping_add(b),
                Op::Subtract => a.wrapping_sub(b),
                Op::Left => a.wrapping_shl(shift(b).ok_or(tr("a shift out of range"))?),
                Op::Right => a.wrapping_shr(shift(b).ok_or(tr("a shift out of range"))?),
                Op::And => a & b,
                Op::Xor => a ^ b,
                Op::Or => a | b,
                Op::Equal => i128::from(a == b),
                Op::NotEqual => i128::from(a != b),
                Op::Less => i128::from(a < b),
                Op::LessEqual => i128::from(a <= b),
                Op::Greater => i128::from(a > b),
                Op::GreaterEqual => i128::from(a >= b),
                Op::Both | Op::Either => i128::from(b != 0),
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scope of named values over the bytes 0, 1, 2 and so on.
    struct Fields;

    impl Scope for Fields {
        fn path(&self, parts: &[Part]) -> Result<i128, String> {
            match parts {
                [Part::Name(a)] if a == "magic" => Ok(0x20b),
                [Part::Name(a), Part::Name(b)] if a == "header" && b == "count" => Ok(3),
                [Part::Name(a), Part::Offset] if a == "header" => Ok(100),
                [Part::Name(a), Part::End] if a == "header" => Ok(124),
                _ => Err(format!("no field {parts:?}")),
            }
        }

        fn here(&self) -> i128 {
            40
        }

        fn read(&mut self, width: u8, at: u64) -> Result<i128, String> {
            // Little-endian bytes whose values are their offsets.
            Ok((0..u64::from(width))
                .fold(0i128, |v, i| v | (i128::from((at + i) as u8) << (8 * i))))
        }
    }

    fn value(text: &str) -> Result<i128, String> {
        eval(&parse(text)?, &mut Fields)
    }

    #[test]
    fn the_operators_go_by_the_precedence_of_rust() {
        assert_eq!(value("1 + 2 * 3"), Ok(7));
        assert_eq!(value("(1 + 2) * 3"), Ok(9));
        assert_eq!(value("0x8000000000000000 >> 63 == 1"), Ok(1));
        assert_eq!(value("0xff & 0x0f | 0x30"), Ok(0x3f));
        assert_eq!(value("1 << 4 + 1"), Ok(32), "+ binds before <<");
        assert_eq!(value("magic == 0x10b || magic == 0x20b"), Ok(1));
        assert_eq!(value("!0 && -1 < 0"), Ok(1));
        assert_eq!(value("7 % 4 - 10 / 3"), Ok(0));
        assert_eq!(value("1_000"), Ok(1000));
    }

    #[test]
    fn paths_and_reads_come_from_the_scope() {
        assert_eq!(value("header.@offset + header.count * 8"), Ok(124));
        assert_eq!(value("header.@end"), Ok(124));
        assert_eq!(value("@here"), Ok(40));
        assert_eq!(value("u16(@here)"), Ok(0x2928));
        assert_eq!(value("u8(2) + u32(0)"), Ok(2 + 0x0302_0100));
        assert!(value("nothing").is_err());
    }

    #[test]
    fn a_wrong_expression_is_a_message() {
        for bad in [
            "1 +", "(1", "1 2", "a.", "@what", "1 / 0", "1 << 200", "#", "u16(1",
        ] {
            assert!(value(bad).is_err(), "{bad}");
        }
        assert!(parse(&"(".repeat(100)).is_err(), "too deep");
        assert!(parse(&"1+".repeat(600)).is_err(), "too long");
        assert_eq!(value("0 && nothing"), Ok(0), "the right side is not read");
    }
}
