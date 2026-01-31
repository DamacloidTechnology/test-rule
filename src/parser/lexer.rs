// src/parser/lexer.rs
//! Lexical analyzer (tokenizer) for the rule DSL

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Rule,
    Function,
    If,
    Else,
    Return,
    True,
    False,
    Null,
    
    // Identifiers and literals
    Identifier(String),
    Number(f64),
    Integer(i64),
    String(String),
    
    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    
    EqEq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
    
    AndAnd,
    OrOr,
    Not,
    
    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    
    Comma,
    Semicolon,
    Colon,
    Dot,
    Assign,
    
    // Special
    Eof,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Identifier(s) => write!(f, "identifier '{}'", s),
            Token::Number(n) => write!(f, "number {}", n),
            Token::Integer(n) => write!(f, "integer {}", n),
            Token::String(s) => write!(f, "string \"{}\"", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Lexer error at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for LexError {}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }
    
    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace_and_comments();
        
        if self.is_at_end() {
            return Ok(Token::Eof);
        }
        
        let ch = self.current_char();
        
        // Single character tokens
        match ch {
            '(' => {
                self.advance();
                return Ok(Token::LeftParen);
            }
            ')' => {
                self.advance();
                return Ok(Token::RightParen);
            }
            '{' => {
                self.advance();
                return Ok(Token::LeftBrace);
            }
            '}' => {
                self.advance();
                return Ok(Token::RightBrace);
            }
            '[' => {
                self.advance();
                return Ok(Token::LeftBracket);
            }
            ']' => {
                self.advance();
                return Ok(Token::RightBracket);
            }
            ',' => {
                self.advance();
                return Ok(Token::Comma);
            }
            ';' => {
                self.advance();
                return Ok(Token::Semicolon);
            }
            ':' => {
                self.advance();
                return Ok(Token::Colon);
            }
            '.' => {
                self.advance();
                return Ok(Token::Dot);
            }
            '+' => {
                self.advance();
                return Ok(Token::Plus);
            }
            '-' => {
                self.advance();
                return Ok(Token::Minus);
            }
            '*' => {
                self.advance();
                return Ok(Token::Star);
            }
            '/' => {
                self.advance();
                return Ok(Token::Slash);
            }
            '%' => {
                self.advance();
                return Ok(Token::Percent);
            }
            '=' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    return Ok(Token::EqEq);
                }
                return Ok(Token::Assign);
            }
            '!' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    return Ok(Token::NotEq);
                }
                return Ok(Token::Not);
            }
            '>' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    return Ok(Token::Gte);
                }
                return Ok(Token::Gt);
            }
            '<' => {
                self.advance();
                if self.current_char() == '=' {
                    self.advance();
                    return Ok(Token::Lte);
                }
                return Ok(Token::Lt);
            }
            '&' => {
                self.advance();
                if self.current_char() == '&' {
                    self.advance();
                    return Ok(Token::AndAnd);
                }
                return Err(self.error("Expected '&&'"));
            }
            '|' => {
                self.advance();
                if self.current_char() == '|' {
                    self.advance();
                    return Ok(Token::OrOr);
                }
                return Err(self.error("Expected '||'"));
            }
            '"' => return self.read_string(),
            _ => {}
        }
        
        // Numbers
        if ch.is_ascii_digit() {
            return self.read_number();
        }
        
        // Identifiers and keywords
        if ch.is_ascii_alphabetic() || ch == '_' {
            return self.read_identifier();
        }
        
        Err(self.error(&format!("Unexpected character: '{}'", ch)))
    }
    
    fn read_identifier(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        
        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        
        let identifier: String = self.input[start..self.position].iter().collect();
        
        let token = match identifier.as_str() {
            "rule" => Token::Rule,
            "function" => Token::Function,
            "if" => Token::If,
            "else" => Token::Else,
            "return" => Token::Return,
            "true" => Token::True,
            "false" => Token::False,
            "null" => Token::Null,
            _ => Token::Identifier(identifier),
        };
        
        Ok(token)
    }
    
    fn read_number(&mut self) -> Result<Token, LexError> {
        let start = self.position;
        let mut has_dot = false;
        
        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_ascii_digit() {
                self.advance();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }
        
        let num_str: String = self.input[start..self.position].iter().collect();
        
        if has_dot {
            let num = num_str.parse::<f64>()
                .map_err(|_| self.error(&format!("Invalid float: {}", num_str)))?;
            Ok(Token::Number(num))
        } else {
            let num = num_str.parse::<i64>()
                .map_err(|_| self.error(&format!("Invalid integer: {}", num_str)))?;
            Ok(Token::Integer(num))
        }
    }
    
    fn read_string(&mut self) -> Result<Token, LexError> {
        self.advance(); // consume opening "
        
        let start = self.position;
        let mut result = String::new();
        
        while !self.is_at_end() && self.current_char() != '"' {
            let ch = self.current_char();
            
            if ch == '\\' {
                self.advance();
                if self.is_at_end() {
                    return Err(self.error("Unterminated string"));
                }
                
                let escaped = match self.current_char() {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    c => c,
                };
                
                result.push(escaped);
                self.advance();
            } else {
                result.push(ch);
                self.advance();
            }
        }
        
        if self.is_at_end() {
            return Err(self.error("Unterminated string"));
        }
        
        self.advance(); // consume closing "
        
        Ok(Token::String(result))
    }
    
    fn skip_whitespace_and_comments(&mut self) {
        while !self.is_at_end() {
            let ch = self.current_char();
            
            if ch.is_whitespace() {
                if ch == '\n' {
                    self.line += 1;
                    self.column = 1;
                } else {
                    self.column += 1;
                }
                self.position += 1;
            } else if ch == '/' && self.peek() == Some('/') {
                // Single-line comment
                while !self.is_at_end() && self.current_char() != '\n' {
                    self.advance();
                }
            } else if ch == '/' && self.peek() == Some('*') {
                // Multi-line comment
                self.advance(); // /
                self.advance(); // *
                
                while !self.is_at_end() {
                    if self.current_char() == '*' && self.peek() == Some('/') {
                        self.advance(); // *
                        self.advance(); // /
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }
    
    fn current_char(&self) -> char {
        self.input[self.position]
    }
    
    fn peek(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }
    
    fn advance(&mut self) {
        if !self.is_at_end() {
            if self.current_char() == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.position += 1;
        }
    }
    
    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }
    
    fn error(&self, message: &str) -> LexError {
        LexError {
            message: message.to_string(),
            line: self.line,
            column: self.column,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let mut lexer = Lexer::new("rule if else ( ) { } + - * /");
        
        assert_eq!(lexer.next_token().unwrap(), Token::Rule);
        assert_eq!(lexer.next_token().unwrap(), Token::If);
        assert_eq!(lexer.next_token().unwrap(), Token::Else);
        assert_eq!(lexer.next_token().unwrap(), Token::LeftParen);
        assert_eq!(lexer.next_token().unwrap(), Token::RightParen);
        assert_eq!(lexer.next_token().unwrap(), Token::LeftBrace);
        assert_eq!(lexer.next_token().unwrap(), Token::RightBrace);
        assert_eq!(lexer.next_token().unwrap(), Token::Plus);
        assert_eq!(lexer.next_token().unwrap(), Token::Minus);
        assert_eq!(lexer.next_token().unwrap(), Token::Star);
        assert_eq!(lexer.next_token().unwrap(), Token::Slash);
    }

    #[test]
    fn test_numbers() {
        let mut lexer = Lexer::new("42 3.14");
        
        assert_eq!(lexer.next_token().unwrap(), Token::Integer(42));
        assert_eq!(lexer.next_token().unwrap(), Token::Number(3.14));
    }

    #[test]
    fn test_strings() {
        let mut lexer = Lexer::new(r#""hello" "world\n""#);
        
        assert_eq!(lexer.next_token().unwrap(), Token::String("hello".to_string()));
        assert_eq!(lexer.next_token().unwrap(), Token::String("world\n".to_string()));
    }

    #[test]
    fn test_identifiers() {
        let mut lexer = Lexer::new("profile txn_count _test");
        
        match lexer.next_token().unwrap() {
            Token::Identifier(s) => assert_eq!(s, "profile"),
            _ => panic!("Expected identifier"),
        }
    }

    #[test]
    fn test_comments() {
        let mut lexer = Lexer::new("rule // comment\nif /* block comment */ else");
        
        assert_eq!(lexer.next_token().unwrap(), Token::Rule);
        assert_eq!(lexer.next_token().unwrap(), Token::If);
        assert_eq!(lexer.next_token().unwrap(), Token::Else);
    }
}
