// src/parser/parser.rs
//! Parser that converts tokens into an Abstract Syntax Tree

use super::ast::*;
use super::lexer::{Lexer, LexError, Token};
use std::fmt;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

impl From<LexError> for ParseError {
    fn from(err: LexError) -> Self {
        ParseError {
            message: err.to_string(),
        }
    }
}

pub struct Parser {
    lexer: Lexer,
    current_token: Token,
}

impl Parser {
    pub fn new(input: &str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(input);
        let current_token = lexer.next_token()?;

        Ok(Self {
            lexer,
            current_token,
        })
    }

    pub fn parse(&mut self) -> Result<Program, ParseError> {
        let mut functions = Vec::new();
        let mut rules = Vec::new();

        while self.current_token != Token::Eof {
            match &self.current_token {
                Token::Function => {
                    functions.push(self.parse_function()?);
                }
                Token::Rule => {
                    rules.push(self.parse_rule()?);
                }
                _ => {
                    return Err(ParseError {
                        message: format!("Expected 'function' or 'rule', got {}", self.current_token),
                    });
                }
            }
        }

        Ok(Program { functions, rules })
    }

    fn parse_function(&mut self) -> Result<FunctionNode, ParseError> {
        self.expect(Token::Function)?;

        let name = self.expect_identifier()?;

        self.expect(Token::LeftParen)?;

        let mut params = Vec::new();
        if self.current_token != Token::RightParen {
            loop {
                params.push(self.expect_identifier()?);

                if self.current_token == Token::Comma {
                    self.advance()?;
                } else {
                    break;
                }
            }
        }

        self.expect(Token::RightParen)?;
        self.expect(Token::LeftBrace)?;

        let body = self.parse_block()?;

        self.expect(Token::RightBrace)?;

        Ok(FunctionNode { name, params, body })
    }

    fn parse_rule(&mut self) -> Result<RuleNode, ParseError> {
        self.expect(Token::Rule)?;

        let id = self.expect_string()?;

        self.expect(Token::LeftBrace)?;

        // Parse rule metadata
        let mut priority = 100;
        let mut enabled = true;

        // Look for priority and enabled fields
        while matches!(self.current_token, Token::Identifier(_)) {
            let field_name = self.expect_identifier()?;
            self.expect(Token::Colon)?;

            match field_name.as_str() {
                "priority" => {
                    if let Token::Integer(n) = self.current_token {
                        priority = n as i32;
                        self.advance()?;
                    } else {
                        return Err(ParseError {
                            message: "Expected integer for priority".to_string(),
                        });
                    }
                }
                "enabled" => {
                    match self.current_token {
                        Token::True => {
                            enabled = true;
                            self.advance()?;
                        }
                        Token::False => {
                            enabled = false;
                            self.advance()?;
                        }
                        _ => {
                            return Err(ParseError {
                                message: "Expected true or false for enabled".to_string(),
                            });
                        }
                    }
                }
                _ => {
                    return Err(ParseError {
                        message: format!("Unknown rule field: {}", field_name),
                    });
                }
            }

            // Skip optional comma
            if self.current_token == Token::Comma {
                self.advance()?;
            }
        }

        // Parse rule body (statements)
        let body = self.parse_block()?;

        self.expect(Token::RightBrace)?;

        Ok(RuleNode {
            id,
            priority,
            enabled,
            body,
        })
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();

        while self.current_token != Token::RightBrace && self.current_token != Token::Eof {
            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match &self.current_token {
            Token::If => self.parse_if_statement(),
            Token::Return => {
                self.advance()?;
                if self.current_token == Token::Semicolon {
                    self.advance()?;
                }
                Ok(Statement::Return)
            }
            Token::Identifier(name) => {
                let name_clone = name.clone();
                self.advance()?;

                // Handle variable declaration: `let <ident> = <expr>;`
                if name_clone == "let" {
                    // After advancing, current_token should be the variable name
                    let var_name = self.expect_identifier()?;
                    self.expect(Token::Assign)?;
                    let value = self.parse_expression()?;
                    if self.current_token == Token::Semicolon {
                        self.advance()?;
                    }
                    return Ok(Statement::Assignment { target: var_name, value });
                }

                // Check if it's an assignment or function/action call
                if self.current_token == Token::Dot {
                    // Could be profile.field = value or object.method()
                    self.advance()?;
                    let field = self.expect_identifier()?;

                    if self.current_token == Token::Assign {
                        // Assignment: profile.field = value
                        self.advance()?;
                        let value = self.parse_expression()?;

                        if self.current_token == Token::Semicolon {
                            self.advance()?;
                        }

                        Ok(Statement::Assignment {
                            target: format!("{}.{}", name_clone, field),
                            value,
                        })
                    } else {
                        // Method call or other expression
                        return Err(ParseError {
                            message: "Expected assignment or method call".to_string(),
                        });
                    }
                } else if self.current_token == Token::LeftParen {
                    // Function/action call
                    self.advance()?;

                    let args = self.parse_argument_list()?;

                    self.expect(Token::RightParen)?;

                    if self.current_token == Token::Semicolon {
                        self.advance()?;
                    }

                    // Distinguish between built-in actions and user-defined functions.
                    // Built-in actions: createCase, createComment, sendAuthAdvise, setFraudScore, setDecision
                    match name_clone.as_str() {
                        "createCase" | "createComment" | "sendAuthAdvise" | "setFraudScore" | "setDecision" => {
                            Ok(Statement::ActionCall { action: name_clone, args })
                        }
                        _ => {
                            // Treat as a function call expression (so compiler emits CallGlobal)
                            Ok(Statement::Expression(Expression::FunctionCall { name: name_clone, args }))
                        }
                    }
                } else if self.current_token == Token::Assign {
                    // Simple variable assignment
                    self.advance()?;
                    let value = self.parse_expression()?;

                    if self.current_token == Token::Semicolon {
                        self.advance()?;
                    }

                    Ok(Statement::Assignment {
                        target: name_clone,
                        value,
                    })
                } else {
                    Err(ParseError {
                        message: format!("Unexpected token after identifier: {}", self.current_token),
                    })
                }
            }
            _ => Err(ParseError {
                message: format!("Unexpected statement: {}", self.current_token),
            }),
        }
    }

    fn parse_if_statement(&mut self) -> Result<Statement, ParseError> {
        self.expect(Token::If)?;
        self.expect(Token::LeftParen)?;

        let condition = self.parse_expression()?;

        self.expect(Token::RightParen)?;
        self.expect(Token::LeftBrace)?;

        let then_block = self.parse_block()?;

        self.expect(Token::RightBrace)?;

        let else_block = if self.current_token == Token::Else {
            self.advance()?;
            self.expect(Token::LeftBrace)?;

            let block = self.parse_block()?;

            self.expect(Token::RightBrace)?;

            Some(block)
        } else {
            None
        };

        Ok(Statement::IfStatement {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_logical_and()?;

        while self.current_token == Token::OrOr {
            self.advance()?;
            let right = self.parse_logical_and()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_equality()?;

        while self.current_token == Token::AndAnd {
            self.advance()?;
            let right = self.parse_equality()?;
            left = Expression::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison()?;

        loop {
            let op = match self.current_token {
                Token::EqEq => BinaryOp::Eq,
                Token::NotEq => BinaryOp::Ne,
                _ => break,
            };

            self.advance()?;
            let right = self.parse_comparison()?;

            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_addition()?;

        loop {
            let op = match self.current_token {
                Token::Gt => BinaryOp::Gt,
                Token::Gte => BinaryOp::Gte,
                Token::Lt => BinaryOp::Lt,
                Token::Lte => BinaryOp::Lte,
                _ => break,
            };

            self.advance()?;
            let right = self.parse_addition()?;

            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_multiplication()?;

        loop {
            let op = match self.current_token {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => break,
            };

            self.advance()?;
            let right = self.parse_multiplication()?;

            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.current_token {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                Token::Percent => BinaryOp::Mod,
                _ => break,
            };

            self.advance()?;
            let right = self.parse_unary()?;

            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        match self.current_token {
            Token::Not => {
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(Expression::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                })
            }
            Token::Minus => {
                self.advance()?;
                let operand = self.parse_unary()?;
                Ok(Expression::Unary {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.current_token {
                Token::Dot => {
                    self.advance()?;
                    let field = self.expect_identifier()?;

                    // Check if it's a method call
                    if self.current_token == Token::LeftParen {
                        self.advance()?;
                        let args = self.parse_argument_list()?;
                        self.expect(Token::RightParen)?;

                        expr = Expression::MethodCall {
                            object: Box::new(expr),
                            method: field,
                            args,
                        };
                    } else {
                        // Simple field access
                        if let Expression::Variable(obj) = expr {
                            expr = Expression::FieldAccess {
                                object: obj,
                                field,
                            };
                        } else {
                            return Err(ParseError {
                                message: "Invalid field access".to_string(),
                            });
                        }
                    }
                }
                Token::LeftBracket => {
                    self.advance()?;
                    let index = self.parse_expression()?;
                    self.expect(Token::RightBracket)?;

                    expr = Expression::ArrayAccess {
                        array: Box::new(expr),
                        index: Box::new(index),
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match &self.current_token {
            Token::True => {
                self.advance()?;
                Ok(Expression::Literal(Literal::Bool(true)))
            }
            Token::False => {
                self.advance()?;
                Ok(Expression::Literal(Literal::Bool(false)))
            }
            Token::Null => {
                self.advance()?;
                Ok(Expression::Literal(Literal::Null))
            }
            Token::Integer(n) => {
                let val = *n;
                self.advance()?;
                Ok(Expression::Literal(Literal::Int(val)))
            }
            Token::Number(n) => {
                let val = *n;
                self.advance()?;
                Ok(Expression::Literal(Literal::Float(val)))
            }
            Token::String(s) => {
                let val = s.clone();
                self.advance()?;
                Ok(Expression::Literal(Literal::String(val)))
            }
            Token::Identifier(name) => {
                let name_clone = name.clone();
                self.advance()?;

                // Check if it's a function call
                if self.current_token == Token::LeftParen {
                    self.advance()?;
                    let args = self.parse_argument_list()?;
                    self.expect(Token::RightParen)?;

                    Ok(Expression::FunctionCall {
                        name: name_clone,
                        args,
                    })
                } else {
                    Ok(Expression::Variable(name_clone))
                }
            }
            Token::LeftParen => {
                self.advance()?;
                let expr = self.parse_expression()?;
                self.expect(Token::RightParen)?;
                Ok(expr)
            }
            _ => Err(ParseError {
                message: format!("Unexpected token in expression: {}", self.current_token),
            }),
        }
    }

    fn parse_argument_list(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut args = Vec::new();

        if self.current_token != Token::RightParen {
            loop {
                args.push(self.parse_expression()?);

                if self.current_token == Token::Comma {
                    self.advance()?;
                } else {
                    break;
                }
            }
        }

        Ok(args)
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        if std::mem::discriminant(&self.current_token) == std::mem::discriminant(&expected) {
            self.advance()?;
            Ok(())
        } else {
            Err(ParseError {
                message: format!("Expected {:?}, got {}", expected, self.current_token),
            })
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match &self.current_token {
            Token::Identifier(name) => {
                let result = name.clone();
                self.advance()?;
                Ok(result)
            }
            _ => Err(ParseError {
                message: format!("Expected identifier, got {}", self.current_token),
            }),
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match &self.current_token {
            Token::String(s) => {
                let result = s.clone();
                self.advance()?;
                Ok(result)
            }
            _ => Err(ParseError {
                message: format!("Expected string, got {}", self.current_token),
            }),
        }
    }

    fn advance(&mut self) -> Result<(), ParseError> {
        self.current_token = self.lexer.next_token()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let input = r#"
            rule "test" {
                priority: 100,
                if (txn.amount > 1000) {
                    setFraudScore(0.8);
                }
            }
        "#;

        let mut parser = Parser::new(input).unwrap();
        let program = parser.parse().unwrap();

        assert_eq!(program.rules.len(), 1);
        assert_eq!(program.rules[0].id, "test");
        assert_eq!(program.rules[0].priority, 100);
    }

    #[test]
    fn test_parse_if_else() {
        let input = r#"
            rule "test" {
                priority: 100,
                if (txn.amount > 1000) {
                    setFraudScore(0.9);
                } else {
                    setFraudScore(0.1);
                }
            }
        "#;

        let mut parser = Parser::new(input).unwrap();
        let program = parser.parse().unwrap();

        assert_eq!(program.rules.len(), 1);

        let stmt = &program.rules[0].body[0];
        if let Statement::IfStatement { else_block, .. } = stmt {
            assert!(else_block.is_some());
        } else {
            panic!("Expected if statement");
        }
    }

    #[test]
    fn test_parse_function() {
        let input = r#"
            function updateCounter(profile) {
                profile.count = profile.count + 1;
            }
        "#;

        let mut parser = Parser::new(input).unwrap();
        let program = parser.parse().unwrap();

        assert_eq!(program.functions.len(), 1);
        assert_eq!(program.functions[0].name, "updateCounter");
        assert_eq!(program.functions[0].params.len(), 1);
    }
}
