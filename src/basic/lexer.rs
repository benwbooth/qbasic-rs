//! BASIC lexer/tokenizer

use std::iter::Peekable;
use std::str::Chars;

/// Token kinds
#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    // Literals
    Integer(i64),
    Float(f64),
    String(String),

    // Identifiers and keywords
    Identifier(String),
    Keyword(Keyword),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Backslash, // Integer division
    Caret,     // Exponentiation
    Equal,
    NotEqual,  // <>
    Less,
    LessEqual,
    Greater,
    GreaterEqual,

    // Punctuation
    LeftParen,
    RightParen,
    Comma,
    Semicolon,
    Colon,
    Hash, // For file numbers

    // Special
    Newline,
    Eof,
}

/// BASIC keywords
#[derive(Clone, Debug, PartialEq)]
pub enum Keyword {
    // Control flow
    If, Then, Else, ElseIf, EndIf,
    For, To, Step, Next,
    While, Wend,
    Do, Loop, Until,
    GoTo, GoSub, Return,
    Select, Case, End,
    Exit,

    // Declarations
    Dim, As, Let,
    Const,
    Sub, Function,
    Shared, Static,
    Type,

    // Data types
    Integer, Long, Single, Double, StringType,

    // Data
    Data, Read, Restore,

    // I/O
    Print, Input, Open, Close, Write,
    Line, Get, Put,
    Append, Output, Random, Binary,

    // Graphics
    Screen, Cls, Color, Locate,
    Pset, Preset, Circle, Paint,
    Draw, View, Window,
    Palette, Bezier,

    // Logical operators
    And, Or, Not, Xor, Eqv, Imp, Mod,

    // Other
    Rem,
    Def, Fn,
    On, Error, Resume,
    Call,
    Swap,
    Beep, Sound, Play,
    Sleep,
    Randomize,
    Stop,
}

/// A token with position info
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }
}

/// The lexer
pub struct Lexer<'a> {
    input: Peekable<Chars<'a>>,
    line: usize,
    column: usize,
    current_char: Option<char>,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Self {
            input: input.chars().peekable(),
            line: 1,
            column: 1,
            current_char: None,
        };
        lexer.advance();
        lexer
    }

    fn advance(&mut self) -> Option<char> {
        let prev = self.current_char;
        self.current_char = self.input.next();
        if let Some(c) = self.current_char {
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
        prev
    }

    fn peek(&mut self) -> Option<char> {
        self.current_char
    }

    fn peek_next(&mut self) -> Option<char> {
        self.input.peek().copied()
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self) -> TokenKind {
        let mut num_str = String::new();
        let mut is_float = false;

        // Integer part
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Decimal part
        if self.peek() == Some('.') {
            if let Some(next) = self.peek_next() {
                if next.is_ascii_digit() {
                    is_float = true;
                    num_str.push('.');
                    self.advance();
                    while let Some(c) = self.peek() {
                        if c.is_ascii_digit() {
                            num_str.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Exponent part
        if let Some(c) = self.peek() {
            if c == 'E' || c == 'e' || c == 'D' || c == 'd' {
                is_float = true;
                num_str.push('E');
                self.advance();

                if let Some(sign) = self.peek() {
                    if sign == '+' || sign == '-' {
                        num_str.push(sign);
                        self.advance();
                    }
                }

                while let Some(c) = self.peek() {
                    if c.is_ascii_digit() {
                        num_str.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }

        // Type suffix
        if let Some(c) = self.peek() {
            match c {
                '#' => {
                    is_float = true;
                    self.advance();
                }
                '!' | '%' | '&' => {
                    self.advance();
                }
                _ => {}
            }
        }

        if is_float {
            TokenKind::Float(num_str.parse().unwrap_or(0.0))
        } else {
            TokenKind::Integer(num_str.parse().unwrap_or(0))
        }
    }

    fn read_string(&mut self) -> TokenKind {
        self.advance(); // Skip opening quote
        let mut s = String::new();

        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                break;
            } else if c == '\n' {
                break; // Unterminated string
            } else {
                s.push(c);
                self.advance();
            }
        }

        TokenKind::String(s)
    }

    fn read_identifier(&mut self) -> TokenKind {
        let mut name = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Check for type suffix
        if let Some(c) = self.peek() {
            if c == '$' || c == '%' || c == '!' || c == '#' || c == '&' {
                name.push(c);
                self.advance();
            }
        }

        // Check if it's a keyword
        if let Some(kw) = Self::get_keyword(&name.to_uppercase()) {
            TokenKind::Keyword(kw)
        } else {
            TokenKind::Identifier(name)
        }
    }

    fn get_keyword(name: &str) -> Option<Keyword> {
        match name {
            // Control flow
            "IF" => Some(Keyword::If),
            "THEN" => Some(Keyword::Then),
            "ELSE" => Some(Keyword::Else),
            "ELSEIF" => Some(Keyword::ElseIf),
            "ENDIF" | "END IF" => Some(Keyword::EndIf),
            "FOR" => Some(Keyword::For),
            "TO" => Some(Keyword::To),
            "STEP" => Some(Keyword::Step),
            "NEXT" => Some(Keyword::Next),
            "WHILE" => Some(Keyword::While),
            "WEND" => Some(Keyword::Wend),
            "DO" => Some(Keyword::Do),
            "LOOP" => Some(Keyword::Loop),
            "UNTIL" => Some(Keyword::Until),
            "GOTO" => Some(Keyword::GoTo),
            "GOSUB" => Some(Keyword::GoSub),
            "RETURN" => Some(Keyword::Return),
            "SELECT" => Some(Keyword::Select),
            "CASE" => Some(Keyword::Case),
            "END" => Some(Keyword::End),
            "EXIT" => Some(Keyword::Exit),

            // Declarations
            "DIM" => Some(Keyword::Dim),
            "AS" => Some(Keyword::As),
            "LET" => Some(Keyword::Let),
            "CONST" => Some(Keyword::Const),
            "SUB" => Some(Keyword::Sub),
            "FUNCTION" => Some(Keyword::Function),
            "SHARED" => Some(Keyword::Shared),
            "STATIC" => Some(Keyword::Static),
            "TYPE" => Some(Keyword::Type),

            // Data types
            "INTEGER" => Some(Keyword::Integer),
            "LONG" => Some(Keyword::Long),
            "SINGLE" => Some(Keyword::Single),
            "DOUBLE" => Some(Keyword::Double),
            "STRING" => Some(Keyword::StringType),

            // Data
            "DATA" => Some(Keyword::Data),
            "READ" => Some(Keyword::Read),
            "RESTORE" => Some(Keyword::Restore),

            // I/O
            "PRINT" => Some(Keyword::Print),
            "INPUT" => Some(Keyword::Input),
            "OPEN" => Some(Keyword::Open),
            "CLOSE" => Some(Keyword::Close),
            "WRITE" => Some(Keyword::Write),
            "LINE" => Some(Keyword::Line),
            "GET" => Some(Keyword::Get),
            "PUT" => Some(Keyword::Put),
            "APPEND" => Some(Keyword::Append),
            "OUTPUT" => Some(Keyword::Output),
            "RANDOM" => Some(Keyword::Random),
            "BINARY" => Some(Keyword::Binary),

            // Graphics
            "SCREEN" => Some(Keyword::Screen),
            "CLS" => Some(Keyword::Cls),
            "COLOR" => Some(Keyword::Color),
            "LOCATE" => Some(Keyword::Locate),
            "PSET" => Some(Keyword::Pset),
            "PRESET" => Some(Keyword::Preset),
            "CIRCLE" => Some(Keyword::Circle),
            "PAINT" => Some(Keyword::Paint),
            "DRAW" => Some(Keyword::Draw),
            "VIEW" => Some(Keyword::View),
            "WINDOW" => Some(Keyword::Window),
            "PALETTE" => Some(Keyword::Palette),
            "BEZIER" => Some(Keyword::Bezier),

            // Logical operators
            "AND" => Some(Keyword::And),
            "OR" => Some(Keyword::Or),
            "NOT" => Some(Keyword::Not),
            "XOR" => Some(Keyword::Xor),
            "EQV" => Some(Keyword::Eqv),
            "IMP" => Some(Keyword::Imp),
            "MOD" => Some(Keyword::Mod),

            // Other
            "REM" => Some(Keyword::Rem),
            "DEF" => Some(Keyword::Def),
            "FN" => Some(Keyword::Fn),
            "ON" => Some(Keyword::On),
            "ERROR" => Some(Keyword::Error),
            "RESUME" => Some(Keyword::Resume),
            "CALL" => Some(Keyword::Call),
            "SWAP" => Some(Keyword::Swap),
            "BEEP" => Some(Keyword::Beep),
            "SOUND" => Some(Keyword::Sound),
            "PLAY" => Some(Keyword::Play),
            "SLEEP" => Some(Keyword::Sleep),
            "RANDOMIZE" => Some(Keyword::Randomize),
            "STOP" => Some(Keyword::Stop),

            _ => None,
        }
    }

    fn skip_comment(&mut self) {
        // Skip until end of line
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let line = self.line;
        let column = self.column;

        let kind = match self.peek() {
            None => TokenKind::Eof,

            Some('\n') => {
                self.advance();
                TokenKind::Newline
            }

            Some('\'') => {
                self.skip_comment();
                TokenKind::Newline
            }

            Some('"') => self.read_string(),

            Some(c) if c.is_ascii_digit() => self.read_number(),

            Some('.') => {
                if let Some(next) = self.peek_next() {
                    if next.is_ascii_digit() {
                        self.read_number()
                    } else {
                        self.advance();
                        TokenKind::Identifier(".".to_string())
                    }
                } else {
                    self.advance();
                    TokenKind::Identifier(".".to_string())
                }
            }

            Some(c) if c.is_alphabetic() || c == '_' => {
                let tok = self.read_identifier();
                // Check for REM comment
                if tok == TokenKind::Keyword(Keyword::Rem) {
                    self.skip_comment();
                    TokenKind::Newline
                } else {
                    tok
                }
            }

            Some('+') => {
                self.advance();
                TokenKind::Plus
            }
            Some('-') => {
                self.advance();
                TokenKind::Minus
            }
            Some('*') => {
                self.advance();
                TokenKind::Star
            }
            Some('/') => {
                self.advance();
                TokenKind::Slash
            }
            Some('\\') => {
                self.advance();
                TokenKind::Backslash
            }
            Some('^') => {
                self.advance();
                TokenKind::Caret
            }
            Some('=') => {
                self.advance();
                TokenKind::Equal
            }
            Some('<') => {
                self.advance();
                match self.peek() {
                    Some('>') => {
                        self.advance();
                        TokenKind::NotEqual
                    }
                    Some('=') => {
                        self.advance();
                        TokenKind::LessEqual
                    }
                    _ => TokenKind::Less,
                }
            }
            Some('>') => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            Some('(') => {
                self.advance();
                TokenKind::LeftParen
            }
            Some(')') => {
                self.advance();
                TokenKind::RightParen
            }
            Some(',') => {
                self.advance();
                TokenKind::Comma
            }
            Some(';') => {
                self.advance();
                TokenKind::Semicolon
            }
            Some(':') => {
                self.advance();
                TokenKind::Colon
            }
            Some('#') => {
                self.advance();
                TokenKind::Hash
            }

            Some(c) => {
                self.advance();
                TokenKind::Identifier(c.to_string())
            }
        };

        Token::new(kind, line, column)
    }

    /// Tokenize entire input
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }
}
