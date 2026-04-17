/// Pratt parser — vil-expr precedence table (§3.2.1).

use crate::ast::*;
use crate::token::Token;

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let tok = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), String> {
        let tok = self.advance();
        if &tok == expected { Ok(()) }
        else { Err(format!("expected {:?}, got {:?} at pos {}", expected, tok, self.pos)) }
    }

    // ── vil-expr Precedence (§3.2.1, low to high) ──
    // 1: ||
    // 2: &&
    // 3: ==, !=
    // 4: <, <=, >, >=
    // 5: in
    // 6: +, -
    // 7: *, /, %
    // Ternary handled separately (lowest, right-assoc)

    pub fn parse(&mut self) -> Result<Expr, String> {
        let expr = self.parse_ternary()?;
        if *self.peek() != Token::Eof {
            // Allow trailing — some callers pass partial
        }
        Ok(expr)
    }

    fn parse_ternary(&mut self) -> Result<Expr, String> {
        let cond = self.parse_or()?;
        if *self.peek() == Token::Question {
            self.advance();
            let then = self.parse_ternary()?; // right-assoc
            self.expect(&Token::Colon)?;
            let else_ = self.parse_ternary()?;
            Ok(Expr::Ternary(Box::new(cond), Box::new(then), Box::new(else_)))
        } else {
            Ok(cond)
        }
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Token::PipePipe | Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary(BinaryOp::Or, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while matches!(self.peek(), Token::AmpAmp | Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::Binary(BinaryOp::And, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_comparison()?;
        loop {
            match self.peek() {
                Token::EqEq => { self.advance(); let r = self.parse_comparison()?; left = Expr::Binary(BinaryOp::Eq, Box::new(left), Box::new(r)); }
                Token::BangEq => { self.advance(); let r = self.parse_comparison()?; left = Expr::Binary(BinaryOp::Neq, Box::new(left), Box::new(r)); }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_membership()?;
        loop {
            match self.peek() {
                Token::Lt => { self.advance(); let r = self.parse_membership()?; left = Expr::Binary(BinaryOp::Lt, Box::new(left), Box::new(r)); }
                Token::Lte => { self.advance(); let r = self.parse_membership()?; left = Expr::Binary(BinaryOp::Lte, Box::new(left), Box::new(r)); }
                Token::Gt => { self.advance(); let r = self.parse_membership()?; left = Expr::Binary(BinaryOp::Gt, Box::new(left), Box::new(r)); }
                Token::Gte => { self.advance(); let r = self.parse_membership()?; left = Expr::Binary(BinaryOp::Gte, Box::new(left), Box::new(r)); }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_membership(&mut self) -> Result<Expr, String> {
        let left = self.parse_additive()?;
        match self.peek() {
            Token::In => {
                self.advance();
                let right = self.parse_set_or_additive()?;
                Ok(Expr::In(Box::new(left), Box::new(right)))
            }
            Token::Not => {
                // NOT IN
                let saved = self.pos;
                self.advance();
                if *self.peek() == Token::In {
                    self.advance();
                    let right = self.parse_set_or_additive()?;
                    Ok(Expr::NotIn(Box::new(left), Box::new(right)))
                } else {
                    self.pos = saved; // backtrack
                    Ok(left)
                }
            }
            Token::Is => {
                // IS NULL / IS NOT NULL
                self.advance();
                if *self.peek() == Token::Not {
                    self.advance();
                    // IS NOT NULL
                    if *self.peek() == Token::Null {
                        self.advance();
                        Ok(Expr::IsNotNull(Box::new(left)))
                    } else {
                        Err("expected NULL after IS NOT".into())
                    }
                } else if *self.peek() == Token::Null {
                    self.advance();
                    Ok(Expr::IsNull(Box::new(left)))
                } else {
                    Err("expected NULL or NOT NULL after IS".into())
                }
            }
            _ => Ok(left),
        }
    }

    /// Parse a set literal {a, b, c}, map {key: value}, or a regular additive expr.
    fn parse_set_or_additive(&mut self) -> Result<Expr, String> {
        if *self.peek() == Token::LBrace {
            self.advance();
            // Empty: {}
            if *self.peek() == Token::RBrace {
                self.advance();
                return Ok(Expr::Map(Vec::new()));
            }
            // Parse first item to determine: Map {key: value} vs Set {a, b, c}
            let first = self.parse_ternary()?;
            if *self.peek() == Token::Colon {
                // Map: {key: value, ...}
                self.advance();
                let val = self.parse_ternary()?;
                let mut entries = vec![(first, val)];
                while *self.peek() == Token::Comma {
                    self.advance();
                    if *self.peek() == Token::RBrace { break; }
                    let key = self.parse_ternary()?;
                    self.expect(&Token::Colon)?;
                    let val = self.parse_ternary()?;
                    entries.push((key, val));
                }
                self.expect(&Token::RBrace)?;
                Ok(Expr::Map(entries))
            } else {
                // Set: {a, b, c} → List for IN/NOT IN
                let mut items = vec![first];
                while *self.peek() == Token::Comma {
                    self.advance();
                    if *self.peek() == Token::RBrace { break; }
                    items.push(self.parse_ternary()?);
                }
                self.expect(&Token::RBrace)?;
                Ok(Expr::List(items))
            }
        } else {
            self.parse_additive()
        }
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;
        loop {
            match self.peek() {
                Token::Plus => { self.advance(); let r = self.parse_multiplicative()?; left = Expr::Binary(BinaryOp::Add, Box::new(left), Box::new(r)); }
                Token::Minus => { self.advance(); let r = self.parse_multiplicative()?; left = Expr::Binary(BinaryOp::Sub, Box::new(left), Box::new(r)); }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_unary()?;
        loop {
            match self.peek() {
                Token::Star => { self.advance(); let r = self.parse_unary()?; left = Expr::Binary(BinaryOp::Mul, Box::new(left), Box::new(r)); }
                Token::Slash => { self.advance(); let r = self.parse_unary()?; left = Expr::Binary(BinaryOp::Div, Box::new(left), Box::new(r)); }
                Token::Percent => { self.advance(); let r = self.parse_unary()?; left = Expr::Binary(BinaryOp::Mod, Box::new(left), Box::new(r)); }
                _ => break,
            }
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Token::Bang | Token::Not => { self.advance(); let e = self.parse_unary()?; Ok(Expr::Unary(UnaryOp::Not, Box::new(e))) }
            Token::Minus => { self.advance(); let e = self.parse_unary()?; Ok(Expr::Unary(UnaryOp::Neg, Box::new(e))) }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek() {
                // Field access: expr.field or method call: expr.method(args)
                Token::Dot => {
                    self.advance();
                    let name = match self.advance() {
                        Token::Ident(s) => s,
                        other => return Err(format!("expected field name after '.', got {:?}", other)),
                    };
                    if *self.peek() == Token::LParen {
                        // Method call
                        self.advance();
                        let args = self.parse_args()?;
                        expr = Expr::MethodCall(Box::new(expr), name, args);
                    } else {
                        expr = Expr::Field(Box::new(expr), name);
                    }
                }
                // Index: expr[index]
                Token::LBracket => {
                    self.advance();
                    let idx = self.parse_ternary()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::Index(Box::new(expr), Box::new(idx));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            Token::Int(n) => { self.advance(); Ok(Expr::Int(n)) }
            Token::Float(n) => { self.advance(); Ok(Expr::Float(n)) }
            Token::Str(s) => { self.advance(); Ok(Expr::String(s)) }
            Token::True => { self.advance(); Ok(Expr::Bool(true)) }
            Token::False => { self.advance(); Ok(Expr::Bool(false)) }
            Token::Null => { self.advance(); Ok(Expr::Null) }

            // Parenthesized
            Token::LParen => {
                self.advance();
                let expr = self.parse_ternary()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }

            // List: [expr, ...]
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();
                if *self.peek() != Token::RBracket {
                    items.push(self.parse_ternary()?);
                    while *self.peek() == Token::Comma {
                        self.advance();
                        if *self.peek() == Token::RBracket { break; } // trailing comma
                        items.push(self.parse_ternary()?);
                    }
                }
                self.expect(&Token::RBracket)?;
                Ok(Expr::List(items))
            }

            // Map/Set handled by parse_set_or_additive — should not reach here
            // But keep as fallback for direct parse_primary calls
            Token::LBrace => {
                self.advance();
                if *self.peek() == Token::RBrace {
                    self.advance();
                    return Ok(Expr::Map(Vec::new()));
                }
                let first = self.parse_ternary()?;
                if *self.peek() == Token::Colon {
                    self.advance();
                    let val = self.parse_ternary()?;
                    let mut entries = vec![(first, val)];
                    while *self.peek() == Token::Comma {
                        self.advance();
                        if *self.peek() == Token::RBrace { break; }
                        let key = self.parse_ternary()?;
                        self.expect(&Token::Colon)?;
                        let v = self.parse_ternary()?;
                        entries.push((key, v));
                    }
                    self.expect(&Token::RBrace)?;
                    Ok(Expr::Map(entries))
                } else {
                    let mut items = vec![first];
                    while *self.peek() == Token::Comma {
                        self.advance();
                        if *self.peek() == Token::RBrace { break; }
                        items.push(self.parse_ternary()?);
                    }
                    self.expect(&Token::RBrace)?;
                    Ok(Expr::List(items))
                }
            }

            // Ident → variable, or function call: name(args)
            Token::Ident(name) => {
                self.advance();
                if *self.peek() == Token::LParen {
                    self.advance();
                    let args = self.parse_args()?;
                    Ok(Expr::FnCall(name, args))
                } else {
                    Ok(Expr::Ident(name))
                }
            }

            other => Err(format!("unexpected {:?} at pos {}", other, self.pos)),
        }
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if *self.peek() != Token::RParen {
            args.push(self.parse_ternary()?);
            while *self.peek() == Token::Comma {
                self.advance();
                args.push(self.parse_ternary()?);
            }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }
}

/// Parse vil-expr expression string → AST.
pub fn parse(input: &str) -> Result<Expr, String> {
    let tokens = crate::token::tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}
