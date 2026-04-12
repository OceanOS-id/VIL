/// Pratt parser — V-CEL precedence table (§3.2.1).

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

    // ── V-CEL Precedence (§3.2.1, low to high) ──
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
        while *self.peek() == Token::PipePipe {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Binary(BinaryOp::Or, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_equality()?;
        while *self.peek() == Token::AmpAmp {
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
        if *self.peek() == Token::In {
            self.advance();
            let right = self.parse_additive()?;
            Ok(Expr::In(Box::new(left), Box::new(right)))
        } else {
            Ok(left)
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
            Token::Bang => { self.advance(); let e = self.parse_unary()?; Ok(Expr::Unary(UnaryOp::Not, Box::new(e))) }
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

            // Map: {key: value, ...}
            Token::LBrace => {
                self.advance();
                let mut entries = Vec::new();
                if *self.peek() != Token::RBrace {
                    loop {
                        let key = self.parse_ternary()?;
                        self.expect(&Token::Colon)?;
                        let val = self.parse_ternary()?;
                        entries.push((key, val));
                        if *self.peek() != Token::Comma { break; }
                        self.advance();
                        if *self.peek() == Token::RBrace { break; }
                    }
                }
                self.expect(&Token::RBrace)?;
                Ok(Expr::Map(entries))
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

/// Parse V-CEL expression string → AST.
pub fn parse(input: &str) -> Result<Expr, String> {
    let tokens = crate::token::tokenize(input)?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}
