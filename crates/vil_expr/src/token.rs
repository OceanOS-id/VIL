/// Tokenizer — vil-expr compatible token set.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    True, False, Null,
    In,                     // keyword `in`
    // vdicl keywords
    And, Or, Not,           // AND, OR, NOT (aliases for &&, ||, !)
    Is,                     // IS (for IS NULL, IS NOT NULL)

    // Operators
    Plus, Minus, Star, Slash, Percent,
    EqEq, BangEq, Lt, Lte, Gt, Gte,
    AmpAmp, PipePipe, Bang,
    Question, Colon,        // ternary ? :

    // Delimiters
    Dot, Comma,
    LParen, RParen,
    LBrace, RBrace,
    LBracket, RBracket,

    Eof,
}

pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        let b = bytes[i];

        // Whitespace
        if b.is_ascii_whitespace() { i += 1; continue; }

        // String: double or single quote
        if b == b'"' || b == b'\'' {
            let quote = b;
            i += 1;
            let start = i;
            let mut s = String::new();
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 1;
                    match bytes[i] {
                        b'n' => s.push('\n'),
                        b't' => s.push('\t'),
                        b'\\' => s.push('\\'),
                        b'\'' => s.push('\''),
                        b'"' => s.push('"'),
                        c => { s.push('\\'); s.push(c as char); }
                    }
                } else {
                    s.push(bytes[i] as char);
                }
                i += 1;
            }
            if i >= len { return Err(format!("unterminated string at {}", start - 1)); }
            i += 1; // skip closing quote
            tokens.push(Token::Str(s));
            continue;
        }

        // Number (with optional decimal suffix `m` for monetary/decimal128)
        if b.is_ascii_digit() {
            let start = i;
            while i < len && bytes[i].is_ascii_digit() { i += 1; }
            if i < len && bytes[i] == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit() {
                i += 1; // skip dot
                while i < len && bytes[i].is_ascii_digit() { i += 1; }
                let s: String = input[start..i].into();
                // Skip trailing `m` suffix (decimal/monetary marker)
                if i < len && bytes[i] == b'm' { i += 1; }
                tokens.push(Token::Float(s.parse().map_err(|_| format!("bad float: {}", s))?));
            } else {
                let s: String = input[start..i].into();
                // Skip trailing `m` suffix — treat as float for monetary values
                if i < len && bytes[i] == b'm' {
                    i += 1;
                    tokens.push(Token::Float(s.parse::<f64>().map_err(|_| format!("bad decimal: {}", s))?));
                } else {
                    tokens.push(Token::Int(s.parse().map_err(|_| format!("bad int: {}", s))?));
                }
            }
            continue;
        }

        // Ident or keyword
        if b.is_ascii_alphabetic() || b == b'_' {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') { i += 1; }
            let word: String = input[start..i].into();
            tokens.push(match word.as_str() {
                "true" => Token::True,
                "false" => Token::False,
                "null" | "NULL" => Token::Null,
                "in" | "IN" => Token::In,
                "AND" => Token::And,
                "OR" => Token::Or,
                "NOT" => Token::Not,
                "IS" => Token::Is,
                _ => Token::Ident(word),
            });
            continue;
        }

        // Two-char operators
        if i + 1 < len {
            match (bytes[i], bytes[i + 1]) {
                (b'=', b'=') => { tokens.push(Token::EqEq); i += 2; continue; }
                (b'!', b'=') => { tokens.push(Token::BangEq); i += 2; continue; }
                (b'<', b'=') => { tokens.push(Token::Lte); i += 2; continue; }
                (b'>', b'=') => { tokens.push(Token::Gte); i += 2; continue; }
                (b'&', b'&') => { tokens.push(Token::AmpAmp); i += 2; continue; }
                (b'|', b'|') => { tokens.push(Token::PipePipe); i += 2; continue; }
                _ => {}
            }
        }

        // Single-char
        let tok = match b {
            b'+' => Token::Plus,
            b'-' => Token::Minus,
            b'*' => Token::Star,
            b'/' => Token::Slash,
            b'%' => Token::Percent,
            b'<' => Token::Lt,
            b'>' => Token::Gt,
            b'!' => Token::Bang,
            b'?' => Token::Question,
            b':' => Token::Colon,
            b'.' => Token::Dot,
            b',' => Token::Comma,
            b'(' => Token::LParen,
            b')' => Token::RParen,
            b'{' => Token::LBrace,
            b'}' => Token::RBrace,
            b'[' => Token::LBracket,
            b']' => Token::RBracket,
            _ => return Err(format!("unexpected '{}' at {}", b as char, i)),
        };
        tokens.push(tok);
        i += 1;
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}
