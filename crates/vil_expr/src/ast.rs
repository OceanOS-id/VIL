/// VIL Expression (vil-expr) compatible AST. Standard Rust — no arena, no zero-copy tricks.

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    // ── Literals (vil-expr §2.1) ──
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Null,

    // ── Collections (vil-expr §2.2) ──
    List(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),

    // ── Access ──
    Ident(String),                              // single identifier
    Field(Box<Expr>, String),                   // expr.field
    Index(Box<Expr>, Box<Expr>),                // expr[index]

    // ── Operators (vil-expr §3.2.1) ──
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),

    // ── Ternary (vil-expr §3.2.1 prec 9) ──
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),   // cond ? then : else

    // ── Membership (vil-expr §3.2.3) ──
    In(Box<Expr>, Box<Expr>),                   // x in [1,2,3]

    // ── Calls ──
    FnCall(String, Vec<Expr>),                  // size(x), has(x), int(x)
    MethodCall(Box<Expr>, String, Vec<Expr>),    // "abc".contains("b")
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,    // !
    Neg,    // - (unary minus)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    // Arithmetic
    Add, Sub, Mul, Div, Mod,
    // Comparison
    Eq, Neq, Lt, Lte, Gt, Gte,
    // Logical
    And, Or,
}
