//! BASIC parser - produces an AST

use crate::basic::lexer::{Token, TokenKind, Keyword};

/// Expression types
#[derive(Clone, Debug)]
pub enum Expr {
    /// Integer literal
    Integer(i64),
    /// Float literal
    Float(f64),
    /// String literal
    String(String),
    /// Variable reference
    Variable(String),
    /// Array access: name(indices)
    ArrayAccess(String, Vec<Expr>),
    /// Binary operation
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    /// Unary operation
    UnaryOp(UnaryOp, Box<Expr>),
    /// Function call
    FunctionCall(String, Vec<Expr>),
    /// Parenthesized expression
    Paren(Box<Expr>),
}

/// Binary operators
#[derive(Clone, Debug)]
pub enum BinOp {
    Add, Sub, Mul, Div, IntDiv, Mod, Pow,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or, Xor, Eqv, Imp,
}

/// Unary operators
#[derive(Clone, Debug)]
pub enum UnaryOp {
    Neg, Not,
}

/// Statement types
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Stmt {
    /// Empty statement (blank line)
    Empty,

    /// Line with optional numeric label
    Label(i64),

    /// Text label (e.g., "StartNewRound:")
    TextLabel(String),

    /// LET assignment: [LET] var = expr
    Let(String, Expr),

    /// Array assignment: arr(indices) = expr
    ArrayLet(String, Vec<Expr>, Expr),

    /// PRINT statement
    Print(Vec<PrintItem>),

    /// INPUT statement: INPUT ["prompt";] var [, var...]
    Input(Option<String>, Vec<String>),

    /// IF/THEN/ELSE
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },

    /// FOR/NEXT loop
    For {
        var: String,
        start: Expr,
        end: Expr,
        step: Option<Expr>,
        body: Vec<Stmt>,
    },

    /// WHILE/WEND loop
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },

    /// DO/LOOP
    DoLoop {
        condition: Option<Expr>,
        is_while: bool,    // WHILE or UNTIL
        is_pre_test: bool, // Condition at DO or at LOOP
        body: Vec<Stmt>,
    },

    /// GOTO line number
    GoTo(i64),

    /// GOTO text label
    GoToLabel(String),

    /// GOSUB line number
    GoSub(i64),

    /// GOSUB text label
    GoSubLabel(String),

    /// RETURN
    Return(Option<Expr>),

    /// DIM statement
    Dim(Vec<DimVar>),

    /// SUB definition
    Sub {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },

    /// FUNCTION definition
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },

    /// CALL sub
    Call(String, Vec<Expr>),

    /// END
    End,

    /// STOP
    Stop,

    /// CLS
    Cls,

    /// SCREEN mode
    Screen(Expr),

    /// COLOR fg [, bg]
    Color(Expr, Option<Expr>),

    /// LOCATE row, col
    Locate(Expr, Expr),

    /// PSET (x, y) [, color]
    Pset(Expr, Expr, Option<Expr>),

    /// LINE (x1,y1)-(x2,y2) [, color] [, B[F]]
    Line {
        x1: Expr,
        y1: Expr,
        x2: Expr,
        y2: Expr,
        color: Option<Expr>,
        box_fill: Option<bool>, // None=line, Some(false)=box, Some(true)=filled box
    },

    /// CIRCLE (x, y), radius [, color] [, start] [, end] [, aspect]
    Circle {
        x: Expr,
        y: Expr,
        radius: Expr,
        color: Option<Expr>,
        start_angle: Option<Expr>,
        end_angle: Option<Expr>,
        aspect: Option<Expr>,
    },

    /// BEZIER (x1, y1)-(cx, cy)-(x2, y2) [, color] [, thickness]
    /// Quadratic bezier curve from (x1,y1) to (x2,y2) with control point (cx,cy)
    Bezier {
        x1: Expr,
        y1: Expr,
        cx: Expr,
        cy: Expr,
        x2: Expr,
        y2: Expr,
        color: Option<Expr>,
        thickness: Option<Expr>,
    },

    /// PAINT (x, y), color
    Paint(Expr, Expr, Expr),

    /// BEEP
    Beep,

    /// SOUND freq, duration
    Sound(Expr, Expr),

    /// SLEEP [seconds]
    Sleep(Option<Expr>),

    /// RANDOMIZE [seed]
    Randomize(Option<Expr>),

    /// DATA values
    Data(Vec<Expr>),

    /// READ vars
    Read(Vec<String>),

    /// RESTORE [line]
    Restore(Option<i64>),

    /// REM comment
    Rem(String),

    /// Expression statement (for immediate mode)
    Expression(Expr),
}

/// Print item (can be expression, separator, or TAB/SPC)
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum PrintItem {
    Expr(Expr),
    Tab(Expr),
    Spc(Expr),
    Comma,
    Semicolon,
}

/// DIM variable with optional size
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct DimVar {
    pub name: String,
    pub dimensions: Vec<Expr>,
    pub var_type: Option<VarType>,
}

/// Variable types
#[derive(Clone, Debug)]
pub enum VarType {
    Integer,
    Long,
    Single,
    Double,
    String,
}

/// Parser for BASIC
pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    /// Context stack for better error messages
    context_stack: Vec<&'static str>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0, context_stack: Vec::new() }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token {
            kind: TokenKind::Eof,
            line: 0,
            column: 0,
        })
    }

    fn peek(&self) -> &TokenKind {
        &self.current().kind
    }

    fn advance(&mut self) -> &Token {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        // Return reference to token at old position
        &self.tokens[self.pos.saturating_sub(1)]
    }

    /// Format an error with line/column and context stack
    fn error(&self, msg: &str) -> String {
        let token = self.current();
        let mut err = format!("Line {}, col {}: {}", token.line, token.column, msg);
        if !self.context_stack.is_empty() {
            err.push_str("\n  Context:");
            for ctx in self.context_stack.iter().rev() {
                err.push_str(&format!("\n    in {}", ctx));
            }
        }
        err
    }

    /// Push parsing context for error messages
    fn push_context(&mut self, ctx: &'static str) {
        self.context_stack.push(ctx);
    }

    /// Pop parsing context
    fn pop_context(&mut self) {
        self.context_stack.pop();
    }

    fn expect(&mut self, expected: TokenKind) -> Result<&Token, String> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(&expected) {
            Ok(self.advance())
        } else {
            Err(self.error(&format!("Expected {:?}, got {:?}", expected, self.peek())))
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), TokenKind::Newline) {
            self.advance();
        }
    }

    /// Check if current token is END followed by IF (lookahead)
    fn is_end_if(&self) -> bool {
        if !matches!(self.peek(), TokenKind::Keyword(Keyword::End)) {
            return false;
        }
        // Look ahead to see if next token is IF
        if let Some(next_token) = self.tokens.get(self.pos + 1) {
            matches!(next_token.kind, TokenKind::Keyword(Keyword::If))
        } else {
            false
        }
    }

    /// Parse the entire program
    pub fn parse(&mut self) -> Result<Vec<Stmt>, String> {
        let mut statements = Vec::new();

        while !matches!(self.peek(), TokenKind::Eof) {
            self.skip_newlines();
            if matches!(self.peek(), TokenKind::Eof) {
                break;
            }

            let stmts = self.parse_statement()?;
            statements.extend(stmts);
        }

        Ok(statements)
    }

    /// Parse a single statement, returning one or two statements if there's a line label
    fn parse_statement(&mut self) -> Result<Vec<Stmt>, String> {
        // Check for line number label
        if let TokenKind::Integer(n) = self.peek().clone() {
            self.advance();
            // Could be followed by a statement on the same line
            if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
                self.advance();
                return Ok(vec![Stmt::Label(n)]);
            }
            // Parse the rest of the statement - return both label AND statement
            let stmt = self.parse_statement_inner()?;
            return Ok(vec![Stmt::Label(n), stmt]);
        }

        // Check for text label (identifier followed by colon)
        if let TokenKind::Identifier(name) = self.peek().clone() {
            // Look ahead for colon
            if self.tokens.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Colon) {
                self.advance(); // consume identifier
                self.advance(); // consume colon
                // Could be followed by a statement on the same line
                if matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
                    return Ok(vec![Stmt::TextLabel(name)]);
                }
                // Parse the rest of the statement - return both label AND statement
                let stmt = self.parse_statement_inner()?;
                return Ok(vec![Stmt::TextLabel(name), stmt]);
            }
        }

        Ok(vec![self.parse_statement_inner()?])
    }

    fn parse_statement_inner(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            TokenKind::Keyword(Keyword::Let) => {
                self.advance();
                self.parse_let()
            }
            TokenKind::Keyword(Keyword::Print) => {
                self.advance();
                self.parse_print()
            }
            TokenKind::Keyword(Keyword::Input) => {
                self.advance();
                self.parse_input()
            }
            TokenKind::Keyword(Keyword::If) => {
                self.advance();
                self.parse_if()
            }
            TokenKind::Keyword(Keyword::For) => {
                self.advance();
                self.parse_for()
            }
            TokenKind::Keyword(Keyword::While) => {
                self.advance();
                self.parse_while()
            }
            TokenKind::Keyword(Keyword::Do) => {
                self.advance();
                self.parse_do_loop()
            }
            TokenKind::Keyword(Keyword::GoTo) => {
                self.advance();
                self.parse_goto()
            }
            TokenKind::Keyword(Keyword::GoSub) => {
                self.advance();
                self.parse_gosub()
            }
            TokenKind::Keyword(Keyword::Return) => {
                self.advance();
                Ok(Stmt::Return(None))
            }
            TokenKind::Keyword(Keyword::Dim) => {
                self.advance();
                self.parse_dim()
            }
            TokenKind::Keyword(Keyword::End) => {
                self.advance();
                Ok(Stmt::End)
            }
            TokenKind::Keyword(Keyword::Stop) => {
                self.advance();
                Ok(Stmt::Stop)
            }
            TokenKind::Keyword(Keyword::Cls) => {
                self.advance();
                Ok(Stmt::Cls)
            }
            TokenKind::Keyword(Keyword::Screen) => {
                self.advance();
                let mode = self.parse_expression()?;
                Ok(Stmt::Screen(mode))
            }
            TokenKind::Keyword(Keyword::Color) => {
                self.advance();
                self.parse_color()
            }
            TokenKind::Keyword(Keyword::Locate) => {
                self.advance();
                self.parse_locate()
            }
            TokenKind::Keyword(Keyword::Pset) => {
                self.advance();
                self.parse_pset()
            }
            TokenKind::Keyword(Keyword::Line) => {
                self.advance();
                self.parse_line()
            }
            TokenKind::Keyword(Keyword::Circle) => {
                self.advance();
                self.parse_circle()
            }
            TokenKind::Keyword(Keyword::Bezier) => {
                self.advance();
                self.parse_bezier()
            }
            TokenKind::Keyword(Keyword::Paint) => {
                self.advance();
                self.parse_paint()
            }
            TokenKind::Keyword(Keyword::Beep) => {
                self.advance();
                Ok(Stmt::Beep)
            }
            TokenKind::Keyword(Keyword::Sound) => {
                self.advance();
                let freq = self.parse_expression()?;
                self.expect(TokenKind::Comma)?;
                let dur = self.parse_expression()?;
                Ok(Stmt::Sound(freq, dur))
            }
            TokenKind::Keyword(Keyword::Sleep) => {
                self.advance();
                let secs = if !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof | TokenKind::Colon) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                Ok(Stmt::Sleep(secs))
            }
            TokenKind::Keyword(Keyword::Randomize) => {
                self.advance();
                let seed = if !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof | TokenKind::Colon) {
                    Some(self.parse_expression()?)
                } else {
                    None
                };
                Ok(Stmt::Randomize(seed))
            }
            TokenKind::Keyword(Keyword::Data) => {
                self.advance();
                self.parse_data()
            }
            TokenKind::Keyword(Keyword::Read) => {
                self.advance();
                self.parse_read()
            }
            TokenKind::Keyword(Keyword::Restore) => {
                self.advance();
                let line = if let TokenKind::Integer(n) = self.peek() {
                    let n = *n;
                    self.advance();
                    Some(n)
                } else {
                    None
                };
                Ok(Stmt::Restore(line))
            }
            TokenKind::Keyword(Keyword::Rem) => {
                // Skip to end of line
                let comment = String::new();
                while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
                    self.advance();
                }
                Ok(Stmt::Rem(comment))
            }
            TokenKind::Keyword(Keyword::Sub) => {
                self.advance();
                self.parse_sub()
            }
            TokenKind::Keyword(Keyword::Function) => {
                self.advance();
                self.parse_function()
            }
            TokenKind::Keyword(Keyword::Call) => {
                self.advance();
                self.parse_call()
            }
            TokenKind::Identifier(_) => {
                // Could be assignment or procedure call
                self.parse_identifier_statement()
            }
            TokenKind::Newline => {
                self.advance();
                Ok(Stmt::Empty)
            }
            _ => {
                // Try to parse as expression (for immediate mode)
                let expr = self.parse_expression()?;
                Ok(Stmt::Expression(expr))
            }
        }
    }

    fn parse_let(&mut self) -> Result<Stmt, String> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Stmt, String> {
        let name = if let TokenKind::Identifier(name) = self.peek().clone() {
            self.advance();
            name
        } else {
            return Err(format!("Expected variable name, got {:?}", self.peek()));
        };

        // Check for array subscript
        if matches!(self.peek(), TokenKind::LeftParen) {
            self.advance();
            let mut indices = Vec::new();
            loop {
                indices.push(self.parse_expression()?);
                if matches!(self.peek(), TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
            self.expect(TokenKind::RightParen)?;
            self.expect(TokenKind::Equal)?;
            let value = self.parse_expression()?;
            Ok(Stmt::ArrayLet(name, indices, value))
        } else {
            self.expect(TokenKind::Equal)?;
            let value = self.parse_expression()?;
            Ok(Stmt::Let(name, value))
        }
    }

    fn parse_identifier_statement(&mut self) -> Result<Stmt, String> {
        // This handles both assignments and procedure calls
        self.parse_assignment()
    }

    fn parse_print(&mut self) -> Result<Stmt, String> {
        let mut items = Vec::new();

        while !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof | TokenKind::Colon) {
            match self.peek() {
                TokenKind::Comma => {
                    self.advance();
                    items.push(PrintItem::Comma);
                }
                TokenKind::Semicolon => {
                    self.advance();
                    items.push(PrintItem::Semicolon);
                }
                _ => {
                    let expr = self.parse_expression()?;
                    items.push(PrintItem::Expr(expr));
                }
            }
        }

        Ok(Stmt::Print(items))
    }

    fn parse_input(&mut self) -> Result<Stmt, String> {
        let prompt = if let TokenKind::String(s) = self.peek().clone() {
            self.advance();
            // Accept either semicolon or comma after prompt
            if matches!(self.peek(), TokenKind::Semicolon | TokenKind::Comma) {
                self.advance();
            }
            Some(s)
        } else {
            None
        };

        let mut vars = Vec::new();
        loop {
            if let TokenKind::Identifier(name) = self.peek().clone() {
                self.advance();
                vars.push(name);
            } else {
                break;
            }
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Stmt::Input(prompt, vars))
    }

    fn parse_if(&mut self) -> Result<Stmt, String> {
        self.push_context("IF statement");
        let condition = self.parse_expression()?;
        self.expect(TokenKind::Keyword(Keyword::Then))?;

        // Check for single-line IF
        if !matches!(self.peek(), TokenKind::Newline | TokenKind::Eof) {
            // Single line IF - parse statement(s) after THEN
            let then_stmt = self.parse_statement_inner()?;
            let else_branch = if matches!(self.peek(), TokenKind::Keyword(Keyword::Else)) {
                self.advance();
                Some(vec![self.parse_statement_inner()?])
            } else {
                None
            };
            self.pop_context();
            return Ok(Stmt::If {
                condition,
                then_branch: vec![then_stmt],
                else_branch,
            });
        }

        // Multi-line IF
        self.advance(); // Skip newline
        let mut then_branch = Vec::new();

        loop {
            self.skip_newlines();
            match self.peek() {
                TokenKind::Keyword(Keyword::Else) | TokenKind::Keyword(Keyword::ElseIf) | TokenKind::Keyword(Keyword::EndIf) => break,
                TokenKind::Keyword(Keyword::End) if self.is_end_if() => break,
                TokenKind::Eof => break,
                _ => {
                    then_branch.extend(self.parse_statement()?);
                }
            }
        }

        let else_branch = if matches!(self.peek(), TokenKind::Keyword(Keyword::ElseIf)) {
            // ELSEIF becomes a nested IF in the else branch
            self.advance(); // consume ELSEIF
            let nested_if = self.parse_elseif()?;
            Some(vec![nested_if])
        } else if matches!(self.peek(), TokenKind::Keyword(Keyword::Else)) {
            self.advance();
            self.skip_newlines();
            let mut else_stmts = Vec::new();
            loop {
                self.skip_newlines();
                match self.peek() {
                    TokenKind::Keyword(Keyword::EndIf) => break,
                    TokenKind::Keyword(Keyword::End) if self.is_end_if() => break,
                    TokenKind::Eof => break,
                    _ => {
                        else_stmts.extend(self.parse_statement()?);
                    }
                }
            }
            // Consume END IF
            self.consume_end_if();
            Some(else_stmts)
        } else {
            // Consume END IF
            self.consume_end_if();
            None
        };

        self.pop_context();
        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    /// Parse ELSEIF clause (similar to IF but handles chained ELSEIF/ELSE)
    fn parse_elseif(&mut self) -> Result<Stmt, String> {
        let condition = self.parse_expression()?;
        self.expect(TokenKind::Keyword(Keyword::Then))?;
        self.skip_newlines();

        let mut then_branch = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek() {
                TokenKind::Keyword(Keyword::Else) | TokenKind::Keyword(Keyword::ElseIf) | TokenKind::Keyword(Keyword::EndIf) => break,
                TokenKind::Keyword(Keyword::End) if self.is_end_if() => break,
                TokenKind::Eof => break,
                _ => {
                    then_branch.extend(self.parse_statement()?);
                }
            }
        }

        let else_branch = if matches!(self.peek(), TokenKind::Keyword(Keyword::ElseIf)) {
            self.advance();
            let nested_if = self.parse_elseif()?;
            Some(vec![nested_if])
        } else if matches!(self.peek(), TokenKind::Keyword(Keyword::Else)) {
            self.advance();
            self.skip_newlines();
            let mut else_stmts = Vec::new();
            loop {
                self.skip_newlines();
                match self.peek() {
                    TokenKind::Keyword(Keyword::EndIf) => break,
                    TokenKind::Keyword(Keyword::End) if self.is_end_if() => break,
                    TokenKind::Eof => break,
                    _ => {
                        else_stmts.extend(self.parse_statement()?);
                    }
                }
            }
            self.consume_end_if();
            Some(else_stmts)
        } else {
            self.consume_end_if();
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    /// Consume END IF tokens
    fn consume_end_if(&mut self) {
        if matches!(self.peek(), TokenKind::Keyword(Keyword::End)) {
            self.advance();
            if matches!(self.peek(), TokenKind::Keyword(Keyword::If)) {
                self.advance();
            }
        } else if matches!(self.peek(), TokenKind::Keyword(Keyword::EndIf)) {
            self.advance();
        }
    }

    fn parse_for(&mut self) -> Result<Stmt, String> {
        self.push_context("FOR loop");
        let var = if let TokenKind::Identifier(name) = self.peek().clone() {
            self.advance();
            name
        } else {
            return Err("Expected variable name after FOR".to_string());
        };

        self.expect(TokenKind::Equal)?;
        let start = self.parse_expression()?;
        self.expect(TokenKind::Keyword(Keyword::To))?;
        let end = self.parse_expression()?;

        let step = if matches!(self.peek(), TokenKind::Keyword(Keyword::Step)) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.skip_newlines();

        let mut body = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), TokenKind::Keyword(Keyword::Next) | TokenKind::Eof) {
                break;
            }
            body.extend(self.parse_statement()?);
        }

        // Consume NEXT [var]
        if matches!(self.peek(), TokenKind::Keyword(Keyword::Next)) {
            self.advance();
            if let TokenKind::Identifier(_) = self.peek() {
                self.advance();
            }
        }

        self.pop_context();
        Ok(Stmt::For { var, start, end, step, body })
    }

    fn parse_while(&mut self) -> Result<Stmt, String> {
        self.push_context("WHILE loop");
        let condition = self.parse_expression()?;
        self.skip_newlines();

        let mut body = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), TokenKind::Keyword(Keyword::Wend) | TokenKind::Eof) {
                break;
            }
            body.extend(self.parse_statement()?);
        }

        if matches!(self.peek(), TokenKind::Keyword(Keyword::Wend)) {
            self.advance();
        }

        self.pop_context();
        Ok(Stmt::While { condition, body })
    }

    fn parse_do_loop(&mut self) -> Result<Stmt, String> {
        self.push_context("DO loop");
        // Check for DO WHILE/UNTIL
        let (pre_condition, is_while) = if matches!(self.peek(), TokenKind::Keyword(Keyword::While)) {
            self.advance();
            (Some(self.parse_expression()?), true)
        } else if matches!(self.peek(), TokenKind::Keyword(Keyword::Until)) {
            self.advance();
            (Some(self.parse_expression()?), false)
        } else {
            (None, true)
        };

        self.skip_newlines();

        let mut body = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), TokenKind::Keyword(Keyword::Loop) | TokenKind::Eof) {
                break;
            }
            body.extend(self.parse_statement()?);
        }

        self.expect(TokenKind::Keyword(Keyword::Loop))?;

        // Check for LOOP WHILE/UNTIL
        if pre_condition.is_none() {
            if matches!(self.peek(), TokenKind::Keyword(Keyword::While)) {
                self.advance();
                let cond = self.parse_expression()?;
                self.pop_context();
                return Ok(Stmt::DoLoop {
                    condition: Some(cond),
                    is_while: true,
                    is_pre_test: false,
                    body,
                });
            } else if matches!(self.peek(), TokenKind::Keyword(Keyword::Until)) {
                self.advance();
                let cond = self.parse_expression()?;
                self.pop_context();
                return Ok(Stmt::DoLoop {
                    condition: Some(cond),
                    is_while: false,
                    is_pre_test: false,
                    body,
                });
            }
        }

        self.pop_context();
        Ok(Stmt::DoLoop {
            condition: pre_condition,
            is_while,
            is_pre_test: true,
            body,
        })
    }

    fn parse_goto(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            TokenKind::Integer(n) => {
                self.advance();
                Ok(Stmt::GoTo(n))
            }
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(Stmt::GoToLabel(name))
            }
            _ => Err("Expected line number or label after GOTO".to_string())
        }
    }

    fn parse_gosub(&mut self) -> Result<Stmt, String> {
        match self.peek().clone() {
            TokenKind::Integer(n) => {
                self.advance();
                Ok(Stmt::GoSub(n))
            }
            TokenKind::Identifier(name) => {
                self.advance();
                Ok(Stmt::GoSubLabel(name))
            }
            _ => Err("Expected line number or label after GOSUB".to_string())
        }
    }

    fn parse_dim(&mut self) -> Result<Stmt, String> {
        let mut vars = Vec::new();

        loop {
            let name = if let TokenKind::Identifier(name) = self.peek().clone() {
                self.advance();
                name
            } else {
                return Err("Expected variable name in DIM".to_string());
            };

            let dimensions = if matches!(self.peek(), TokenKind::LeftParen) {
                self.advance();
                let mut dims = Vec::new();
                loop {
                    dims.push(self.parse_expression()?);
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(TokenKind::RightParen)?;
                dims
            } else {
                Vec::new()
            };

            let var_type = if matches!(self.peek(), TokenKind::Keyword(Keyword::As)) {
                self.advance();
                match self.peek() {
                    TokenKind::Keyword(Keyword::Integer) => {
                        self.advance();
                        Some(VarType::Integer)
                    }
                    TokenKind::Keyword(Keyword::Long) => {
                        self.advance();
                        Some(VarType::Long)
                    }
                    TokenKind::Keyword(Keyword::Single) => {
                        self.advance();
                        Some(VarType::Single)
                    }
                    TokenKind::Keyword(Keyword::Double) => {
                        self.advance();
                        Some(VarType::Double)
                    }
                    TokenKind::Keyword(Keyword::StringType) => {
                        self.advance();
                        Some(VarType::String)
                    }
                    _ => None,
                }
            } else {
                None
            };

            vars.push(DimVar { name, dimensions, var_type });

            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        Ok(Stmt::Dim(vars))
    }

    fn parse_color(&mut self) -> Result<Stmt, String> {
        let fg = self.parse_expression()?;
        let bg = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        Ok(Stmt::Color(fg, bg))
    }

    fn parse_locate(&mut self) -> Result<Stmt, String> {
        let row = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let col = self.parse_expression()?;
        Ok(Stmt::Locate(row, col))
    }

    fn parse_pset(&mut self) -> Result<Stmt, String> {
        self.expect(TokenKind::LeftParen)?;
        let x = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let y = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        let color = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Stmt::Pset(x, y, color))
    }

    fn parse_line(&mut self) -> Result<Stmt, String> {
        self.expect(TokenKind::LeftParen)?;
        let x1 = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let y1 = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        self.expect(TokenKind::Minus)?;
        self.expect(TokenKind::LeftParen)?;
        let x2 = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let y2 = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        let color = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            if matches!(self.peek(), TokenKind::Comma) {
                None
            } else {
                Some(self.parse_expression()?)
            }
        } else {
            None
        };

        let box_fill = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            if let TokenKind::Identifier(s) = self.peek() {
                match s.to_uppercase().as_str() {
                    "B" => {
                        self.advance();
                        Some(false)
                    }
                    "BF" => {
                        self.advance();
                        Some(true)
                    }
                    _ => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Stmt::Line { x1, y1, x2, y2, color, box_fill })
    }

    fn parse_circle(&mut self) -> Result<Stmt, String> {
        self.expect(TokenKind::LeftParen)?;
        let x = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let y = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        self.expect(TokenKind::Comma)?;
        let radius = self.parse_expression()?;

        let color = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // Parse optional start angle
        let start_angle = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // Parse optional end angle
        let end_angle = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // Parse optional aspect ratio
        let aspect = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Stmt::Circle { x, y, radius, color, start_angle, end_angle, aspect })
    }

    /// Parse BEZIER (x1, y1)-(cx, cy)-(x2, y2) [, color] [, thickness]
    fn parse_bezier(&mut self) -> Result<Stmt, String> {
        // First point
        self.expect(TokenKind::LeftParen)?;
        let x1 = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let y1 = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        self.expect(TokenKind::Minus)?;

        // Control point
        self.expect(TokenKind::LeftParen)?;
        let cx = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let cy = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        self.expect(TokenKind::Minus)?;

        // End point
        self.expect(TokenKind::LeftParen)?;
        let x2 = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let y2 = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;

        // Optional color
        let color = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        // Optional thickness
        let thickness = if matches!(self.peek(), TokenKind::Comma) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Stmt::Bezier { x1, y1, cx, cy, x2, y2, color, thickness })
    }

    fn parse_paint(&mut self) -> Result<Stmt, String> {
        self.expect(TokenKind::LeftParen)?;
        let x = self.parse_expression()?;
        self.expect(TokenKind::Comma)?;
        let y = self.parse_expression()?;
        self.expect(TokenKind::RightParen)?;
        self.expect(TokenKind::Comma)?;
        let color = self.parse_expression()?;

        Ok(Stmt::Paint(x, y, color))
    }

    fn parse_data(&mut self) -> Result<Stmt, String> {
        let mut values = Vec::new();
        loop {
            values.push(self.parse_expression()?);
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(Stmt::Data(values))
    }

    fn parse_read(&mut self) -> Result<Stmt, String> {
        let mut vars = Vec::new();
        loop {
            if let TokenKind::Identifier(name) = self.peek().clone() {
                self.advance();
                vars.push(name);
            } else {
                break;
            }
            if matches!(self.peek(), TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(Stmt::Read(vars))
    }

    /// Parse SUB definition
    fn parse_sub(&mut self) -> Result<Stmt, String> {
        // SUB name [(params)]
        let name = if let TokenKind::Identifier(name) = self.peek().clone() {
            self.advance();
            name
        } else {
            return Err("Expected SUB name".to_string());
        };

        // Parse parameters
        let params = if matches!(self.peek(), TokenKind::LeftParen) {
            self.advance();
            self.parse_param_list()?
        } else {
            Vec::new()
        };

        // Skip to newline
        while matches!(self.peek(), TokenKind::Newline) {
            self.advance();
        }

        // Parse body until END SUB
        let mut body = Vec::new();
        loop {
            if matches!(self.peek(), TokenKind::Keyword(Keyword::End)) {
                self.advance();
                if matches!(self.peek(), TokenKind::Keyword(Keyword::Sub)) {
                    self.advance();
                    break;
                } else {
                    body.push(Stmt::End);
                }
            } else if matches!(self.peek(), TokenKind::Eof) {
                return Err("Unexpected end of file in SUB".to_string());
            } else {
                body.extend(self.parse_statement()?);
            }
        }

        Ok(Stmt::Sub { name, params, body })
    }

    /// Parse FUNCTION definition
    fn parse_function(&mut self) -> Result<Stmt, String> {
        // FUNCTION name [(params)]
        let name = if let TokenKind::Identifier(name) = self.peek().clone() {
            self.advance();
            name
        } else {
            return Err("Expected FUNCTION name".to_string());
        };

        // Parse parameters
        let params = if matches!(self.peek(), TokenKind::LeftParen) {
            self.advance();
            self.parse_param_list()?
        } else {
            Vec::new()
        };

        // Skip to newline
        while matches!(self.peek(), TokenKind::Newline) {
            self.advance();
        }

        // Parse body until END FUNCTION
        let mut body = Vec::new();
        loop {
            if matches!(self.peek(), TokenKind::Keyword(Keyword::End)) {
                self.advance();
                if matches!(self.peek(), TokenKind::Keyword(Keyword::Function)) {
                    self.advance();
                    break;
                } else {
                    body.push(Stmt::End);
                }
            } else if matches!(self.peek(), TokenKind::Eof) {
                return Err("Unexpected end of file in FUNCTION".to_string());
            } else {
                body.extend(self.parse_statement()?);
            }
        }

        Ok(Stmt::Function { name, params, body })
    }

    /// Parse parameter list
    fn parse_param_list(&mut self) -> Result<Vec<String>, String> {
        let mut params = Vec::new();

        if !matches!(self.peek(), TokenKind::RightParen) {
            loop {
                if let TokenKind::Identifier(name) = self.peek().clone() {
                    self.advance();
                    params.push(name);
                } else {
                    return Err("Expected parameter name".to_string());
                }

                if matches!(self.peek(), TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.expect(TokenKind::RightParen)?;
        Ok(params)
    }

    /// Parse CALL statement
    fn parse_call(&mut self) -> Result<Stmt, String> {
        // CALL name [(args)]
        let name = if let TokenKind::Identifier(name) = self.peek().clone() {
            self.advance();
            name
        } else {
            return Err("Expected subroutine name".to_string());
        };

        // Parse arguments
        let args = if matches!(self.peek(), TokenKind::LeftParen) {
            self.advance();
            let mut args = Vec::new();
            if !matches!(self.peek(), TokenKind::RightParen) {
                loop {
                    args.push(self.parse_expression()?);
                    if matches!(self.peek(), TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
            self.expect(TokenKind::RightParen)?;
            args
        } else {
            Vec::new()
        };

        Ok(Stmt::Call(name, args))
    }

    /// Parse an expression
    pub fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_and()?;

        while matches!(self.peek(), TokenKind::Keyword(Keyword::Or) | TokenKind::Keyword(Keyword::Xor) | TokenKind::Keyword(Keyword::Eqv) | TokenKind::Keyword(Keyword::Imp)) {
            let op = match self.peek() {
                TokenKind::Keyword(Keyword::Or) => BinOp::Or,
                TokenKind::Keyword(Keyword::Xor) => BinOp::Xor,
                TokenKind::Keyword(Keyword::Eqv) => BinOp::Eqv,
                TokenKind::Keyword(Keyword::Imp) => BinOp::Imp,
                _ => unreachable!(),
            };
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_not()?;

        while matches!(self.peek(), TokenKind::Keyword(Keyword::And)) {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::BinaryOp(Box::new(left), BinOp::And, Box::new(right));
        }

        Ok(left)
    }

    fn parse_not(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), TokenKind::Keyword(Keyword::Not)) {
            self.advance();
            let expr = self.parse_not()?;
            Ok(Expr::UnaryOp(UnaryOp::Not, Box::new(expr)))
        } else {
            self.parse_comparison()
        }
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_additive()?;

        loop {
            let op = match self.peek() {
                TokenKind::Equal => BinOp::Eq,
                TokenKind::NotEqual => BinOp::Ne,
                TokenKind::Less => BinOp::Lt,
                TokenKind::LessEqual => BinOp::Le,
                TokenKind::Greater => BinOp::Gt,
                TokenKind::GreaterEqual => BinOp::Ge,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_mod()?;

        loop {
            let op = match self.peek() {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_mod()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_mod(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_int_div()?;

        while matches!(self.peek(), TokenKind::Keyword(Keyword::Mod)) {
            self.advance();
            let right = self.parse_int_div()?;
            left = Expr::BinaryOp(Box::new(left), BinOp::Mod, Box::new(right));
        }

        Ok(left)
    }

    fn parse_int_div(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_multiplicative()?;

        while matches!(self.peek(), TokenKind::Backslash) {
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinaryOp(Box::new(left), BinOp::IntDiv, Box::new(right));
        }

        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_power()?;

        loop {
            let op = match self.peek() {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let base = self.parse_unary()?;

        if matches!(self.peek(), TokenKind::Caret) {
            self.advance();
            let exponent = self.parse_power()?; // Right associative
            Ok(Expr::BinaryOp(Box::new(base), BinOp::Pow, Box::new(exponent)))
        } else {
            Ok(base)
        }
    }

    fn parse_unary(&mut self) -> Result<Expr, String> {
        match self.peek() {
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_unary()?;
                Ok(Expr::UnaryOp(UnaryOp::Neg, Box::new(expr)))
            }
            TokenKind::Plus => {
                self.advance();
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.peek().clone() {
            TokenKind::Integer(n) => {
                self.advance();
                Ok(Expr::Integer(n))
            }
            TokenKind::Float(n) => {
                self.advance();
                Ok(Expr::Float(n))
            }
            TokenKind::String(s) => {
                self.advance();
                Ok(Expr::String(s))
            }
            TokenKind::Identifier(name) => {
                self.advance();
                // Check for function call or array access
                if matches!(self.peek(), TokenKind::LeftParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !matches!(self.peek(), TokenKind::RightParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if matches!(self.peek(), TokenKind::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(TokenKind::RightParen)?;

                    // Determine if it's a function call or array access
                    // Built-in functions and user functions use FunctionCall
                    // Arrays use ArrayAccess
                    if is_builtin_function(&name) {
                        Ok(Expr::FunctionCall(name, args))
                    } else {
                        Ok(Expr::ArrayAccess(name, args))
                    }
                } else if is_parameterless_function(&name) {
                    // Handle functions that can be called without parentheses (RND, TIMER, etc.)
                    Ok(Expr::FunctionCall(name, vec![]))
                } else {
                    Ok(Expr::Variable(name))
                }
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(TokenKind::RightParen)?;
                Ok(Expr::Paren(Box::new(expr)))
            }
            _ => Err(self.error(&format!("Unexpected token in expression: {:?}", self.peek()))),
        }
    }
}

fn is_builtin_function(name: &str) -> bool {
    let name_upper = name.to_uppercase();
    matches!(name_upper.as_str(),
        "ABS" | "INT" | "FIX" | "SGN" | "SQR" | "SIN" | "COS" | "TAN" | "ATN" | "LOG" | "EXP" | "RND" |
        "LEN" | "LEFT$" | "RIGHT$" | "MID$" | "STR$" | "VAL" | "CHR$" | "ASC" | "INSTR" |
        "UCASE$" | "LCASE$" | "LTRIM$" | "RTRIM$" | "SPACE$" | "STRING$" |
        "CINT" | "CLNG" | "CSNG" | "CDBL" |
        "TIMER" | "DATE$" | "TIME$" | "INKEY$" |
        "PEEK" | "FRE" | "POS" | "CSRLIN" | "POINT"
    )
}

/// Functions that can be called without parentheses (zero-argument functions)
fn is_parameterless_function(name: &str) -> bool {
    let name_upper = name.to_uppercase();
    matches!(name_upper.as_str(),
        "RND" | "TIMER" | "DATE$" | "TIME$" | "INKEY$" | "POS" | "CSRLIN"
    )
}
