use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Let,
    Flag(String),
    Identifier(String),
    Word(String),
    Number(String),
    StringLiteral(String),
    Boolean(bool),
    Fd(u8),
    Equals,
    Pipe,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Dot,
    Comma,
    Semicolon,
    GreaterThan,
    AppendGreaterThan,
    LessThan,
    LessThanLessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    EqualsEquals,
    BangEquals,
    CombinedRedirect,
    Plus,
    Minus,
    Star,
    Slash,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number { value: i64, unit: Option<String> },
    String(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Equals,
    NotEquals,
    Add,
    Subtract,
    Multiply,
    Divide,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectMode {
    Truncate,
    Append,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedirectTarget {
    File(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTNode {
    Let {
        name: String,
        value: Box<ASTNode>,
    },
    Pipeline {
        stages: Vec<Box<ASTNode>>,
    },
    Command {
        name: String,
        args: Vec<ASTNode>,
    },
    Redirect {
        source: Box<ASTNode>,
        fd: u8,
        mode: RedirectMode,
        target: RedirectTarget,
        merge_stderr: bool,
    },
    Block {
        body: Vec<ASTNode>,
    },
    BinaryOp {
        left: Box<ASTNode>,
        op: BinaryOperator,
        right: Box<ASTNode>,
    },
    Variable(String),
    Literal(Literal),
    PropertyAccess {
        target: Box<ASTNode>,
        property: String,
    },
    Identifier(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
}

impl ParseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse Error: {}", self.message)
    }
}

impl std::error::Error for ParseError {}

type Result<T> = std::result::Result<T, ParseError>;

pub fn tokenize_line(input: &str) -> Result<Vec<Token>> {
    Lexer::new(input).tokenize()
}

pub fn parse_line(input: &str) -> Result<ASTNode> {
    let tokens = tokenize_line(input)?;
    let mut parser = Parser::new(tokens);
    let node = parser.parse_statement()?;

    if parser.peek().is_some() {
        Err(ParseError::new("Unexpected input after end of statement"))
    } else {
        Ok(node)
    }
}

struct Lexer<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();

        while let Some(&ch) = self.chars.peek() {
            match ch {
                ch if ch.is_whitespace() => {
                    self.chars.next();
                }
                'a'..='z' | 'A'..='Z' | '_' | '$' => {
                    tokens.push(self.lex_identifier_or_keyword()?);
                }
                '0'..='9' => {
                    tokens.extend(self.lex_number_or_fd()?);
                }
                '&' => {
                    self.chars.next();
                    if matches!(self.chars.peek(), Some('>')) {
                        self.chars.next();
                        tokens.push(Token::CombinedRedirect);
                    } else {
                        return Err(ParseError::new("Unexpected '&' without redirect"));
                    }
                }
                '"' | '\'' => {
                    tokens.push(self.lex_string(ch)?);
                }
                '=' => {
                    self.chars.next();
                    if matches!(self.chars.peek(), Some('=')) {
                        self.chars.next();
                        tokens.push(Token::EqualsEquals);
                    } else {
                        tokens.push(Token::Equals);
                    }
                }
                '|' => {
                    self.chars.next();
                    tokens.push(Token::Pipe);
                }
                '(' => {
                    self.chars.next();
                    tokens.push(Token::LParen);
                }
                ')' => {
                    self.chars.next();
                    tokens.push(Token::RParen);
                }
                '{' => {
                    self.chars.next();
                    tokens.push(Token::LBrace);
                }
                '}' => {
                    self.chars.next();
                    tokens.push(Token::RBrace);
                }
                '.' => {
                    self.chars.next();
                    tokens.push(Token::Dot);
                }
                ',' => {
                    self.chars.next();
                    tokens.push(Token::Comma);
                }
                ';' => {
                    self.chars.next();
                    tokens.push(Token::Semicolon);
                }
                '>' => {
                    self.chars.next();
                    if matches!(self.chars.peek(), Some('>')) {
                        self.chars.next();
                        tokens.push(Token::AppendGreaterThan);
                    } else {
                        tokens.push(Token::GreaterThan);
                    }
                }
                '<' => {
                    self.chars.next();
                    if matches!(self.chars.peek(), Some('<')) {
                        self.chars.next();
                        tokens.push(Token::LessThanLessThan);
                    } else {
                        tokens.push(Token::LessThan);
                    }
                }
                '!' => {
                    self.chars.next();
                    if matches!(self.chars.peek(), Some('=')) {
                        self.chars.next();
                        tokens.push(Token::BangEquals);
                    } else {
                        return Err(ParseError::new("Unexpected '!' without '='"));
                    }
                }
                '+' => {
                    self.chars.next();
                    tokens.push(Token::Plus);
                }
                '-' => {
                    // Could be a flag like `-la`, `--help`, or `-1`.
                    // Only treat a preceding hyphen as a subtraction operator when it is
                    // followed by an actual operator delimiter or end of input.
                    if let Some(second) = {
                        let mut it = self.chars.clone();
                        it.next(); // current '-'
                        it.peek().cloned()
                    } {
                        if !second.is_whitespace()
                            && !matches!(second, '>' | '<' | '|' | '&' | ';' | '(' | ')' | '{' | '}' | '=' | '+' | '*' | '/')
                        {
                            tokens.push(self.lex_flag()?);
                        } else {
                            self.chars.next();
                            tokens.push(Token::Minus);
                        }
                    } else {
                        self.chars.next();
                        tokens.push(Token::Minus);
                    }
                }
                '*' => {
                    self.chars.next();
                    tokens.push(Token::Star);
                }
                '/' => {
                    self.chars.next();
                    tokens.push(Token::Slash);
                }
                _other => {
                    tokens.push(self.lex_word()?);
                }
            }
        }

        Ok(tokens)
    }

    fn lex_identifier_or_keyword(&mut self) -> Result<Token> {
        let mut text = String::new();

        while let Some(&ch) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '$' {
                text.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }

        let token = match text.as_str() {
            "let" => Token::Let,
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            _ => Token::Identifier(text),
        };

        Ok(token)
    }

    fn lex_flag(&mut self) -> Result<Token> {
        let mut text = String::new();
        // consume the leading '-'
        if let Some(ch) = self.chars.next() {
            if ch != '-' {
                // should not happen
            }
            text.push('-');
        }

        // capture optional second '-'
        if let Some(&next) = self.chars.peek() {
            if next == '-' {
                text.push('-');
                self.chars.next();
            }
        }

        while let Some(&ch) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                text.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }

        Ok(Token::Flag(text))
    }

    fn lex_number_or_fd(&mut self) -> Result<Vec<Token>> {
        let mut digits = String::new();

        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_digit() {
                digits.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }

        if digits.is_empty() {
            return Err(ParseError::new("Expected number literal"));
        }

        if matches!(self.chars.peek(), Some(&'>')) || matches!(self.chars.peek(), Some(&'<')) {
            let fd_value = digits.parse::<u8>().map_err(|_| ParseError::new("Invalid file descriptor"))?;
            return Ok(vec![Token::Fd(fd_value)]);
        }

        let mut text = digits;
        while let Some(&ch) = self.chars.peek() {
            if ch.is_ascii_alphanumeric() {
                text.push(ch);
                self.chars.next();
            } else {
                break;
            }
        }

        Ok(vec![Token::Number(text)])
    }

    fn lex_word(&mut self) -> Result<Token> {
        let mut value = String::new();

        while let Some(&ch) = self.chars.peek() {
            if ch.is_whitespace()
                || matches!(ch, '|' | '<' | '>' | '&' | '(' | ')' | '{' | '}' | ',' | ';' | '=' | '+' | '-' | '*' | '/')
            {
                break;
            }

            value.push(ch);
            self.chars.next();
        }

        if value.is_empty() {
            return Err(ParseError::new("Unexpected invalid word"));
        }

        Ok(Token::Word(value))
    }

    fn lex_string(&mut self, quote: char) -> Result<Token> {
        self.chars.next();
        let mut value = String::new();

        while let Some(ch) = self.chars.next() {
            match ch {
                ch if ch == quote => return Ok(Token::StringLiteral(value)),
                '\\' if quote == '"' => {
                    if let Some(next) = self.chars.next() {
                        value.push(next);
                    } else {
                        value.push('\\');
                    }
                }
                other => value.push(other),
            }
        }

        Err(ParseError::new("Unterminated string literal"))
    }
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, position: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn next(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.position);
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    fn expect_identifier(&mut self) -> Result<String> {
        match self.next() {
            Some(Token::Identifier(name)) => Ok(name.clone()),
            Some(other) => Err(ParseError::new(format!("Expected identifier, found {:?}", other))),
            None => Err(ParseError::new("Expected identifier, found end of input")),
        }
    }

    fn expect_token(&mut self, expected: Token) -> Result<()> 
    where
        Token: PartialEq,
    {
        match self.next() {
            Some(token) if *token == expected => Ok(()),
            Some(other) => Err(ParseError::new(format!("Expected {:?}, found {:?}", expected, other))),
            None => Err(ParseError::new(format!("Expected {:?}, found end of input", expected))),
        }
    }

    fn parse_statement(&mut self) -> Result<ASTNode> {
        if matches!(self.peek(), Some(Token::Let)) {
            self.parse_let_statement()
        } else {
            self.parse_pipeline()
        }
    }

    fn parse_let_statement(&mut self) -> Result<ASTNode> {
        self.next();
        let name = self.expect_identifier()?;
        self.expect_token(Token::Equals)?;
        let value = self.parse_expression()?;
        Ok(ASTNode::Let {
            name,
            value: Box::new(value),
        })
    }

    fn parse_pipeline(&mut self) -> Result<ASTNode> {
        let mut stages = vec![Box::new(self.parse_expression()?)];

        while matches!(self.peek(), Some(Token::Pipe)) {
            self.next();
            stages.push(Box::new(self.parse_expression()?));
        }

        if stages.len() == 1 {
            Ok(*stages.remove(0))
        } else {
            Ok(ASTNode::Pipeline { stages })
        }
    }

    fn parse_expression(&mut self) -> Result<ASTNode> {
        let node = self.parse_comparison()?;
        self.parse_command_chain(node)
    }

    fn parse_command_chain(&mut self, node: ASTNode) -> Result<ASTNode> {
        let node = match node {
            ASTNode::Identifier(name) if self.peek_is_command_token() => {
                self.parse_command_from_name(name)?
            }
            ASTNode::Identifier(name) if self.peek_is_end_of_command() => ASTNode::Command {
                name,
                args: Vec::new(),
            },
            other => other,
        };

        if self.peek_redirect_start() {
            self.parse_redirect(node)
        } else {
            Ok(node)
        }
    }

    fn parse_comparison(&mut self) -> Result<ASTNode> {
        let mut left = self.parse_term()?;

        while let Some(op) = self.peek_comparison_operator() {
            self.next();
            let right = self.parse_term()?;
            left = ASTNode::BinaryOp {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn peek_comparison_operator(&self) -> Option<BinaryOperator> {
        match self.peek() {
            Some(Token::GreaterThan) => Some(BinaryOperator::GreaterThan),
            Some(Token::LessThan) => Some(BinaryOperator::LessThan),
            Some(Token::GreaterThanOrEqual) => Some(BinaryOperator::GreaterThanOrEqual),
            Some(Token::LessThanOrEqual) => Some(BinaryOperator::LessThanOrEqual),
            Some(Token::EqualsEquals) => Some(BinaryOperator::Equals),
            Some(Token::BangEquals) => Some(BinaryOperator::NotEquals),
            _ => None,
        }
    }

    fn parse_term(&mut self) -> Result<ASTNode> {
        let mut node = self.parse_factor()?;

        while let Some(op) = self.peek_term_operator() {
            self.next();
            let right = self.parse_factor()?;
            node = ASTNode::BinaryOp {
                left: Box::new(node),
                op,
                right: Box::new(right),
            };
        }

        Ok(node)
    }

    fn peek_term_operator(&self) -> Option<BinaryOperator> {
        match self.peek() {
            Some(Token::Plus) => Some(BinaryOperator::Add),
            Some(Token::Minus) => Some(BinaryOperator::Subtract),
            _ => None,
        }
    }

    fn parse_factor(&mut self) -> Result<ASTNode> {
        let mut node = self.parse_primary()?;

        match node {
            ASTNode::Identifier(_) | ASTNode::Command { .. } => {
                node = self.parse_command_chain(node)?;
            }
            _ => {}
        }

        while let Some(op) = self.peek_factor_operator() {
            self.next();
            let right = self.parse_primary()?;
            node = ASTNode::BinaryOp {
                left: Box::new(node),
                op,
                right: Box::new(right),
            };
        }

        Ok(node)
    }

    fn peek_factor_operator(&self) -> Option<BinaryOperator> {
        match self.peek() {
            Some(Token::Star) => Some(BinaryOperator::Multiply),
            Some(Token::Slash) => Some(BinaryOperator::Divide),
            _ => None,
        }
    }

    fn parse_primary(&mut self) -> Result<ASTNode> {
        let token = self.next().cloned();

        match token {
            Some(Token::Identifier(name)) => self.parse_identifier_expression(name),            Some(Token::Word(name)) => self.parse_identifier_expression(name),            Some(Token::Number(number)) => Ok(ASTNode::Literal(self.parse_number_literal(&number)?)),
            Some(Token::StringLiteral(text)) => Ok(ASTNode::Literal(Literal::String(text))),
            Some(Token::Flag(flag)) => Ok(ASTNode::Literal(Literal::String(flag))),
            Some(Token::Boolean(b)) => Ok(ASTNode::Literal(Literal::Boolean(b))),
            Some(Token::LParen) => {
                let expression = self.parse_expression()?;
                self.expect_token(Token::RParen)?;
                Ok(expression)
            }
            Some(Token::LBrace) => self.parse_block(),
            Some(other) => Err(ParseError::new(format!("Unexpected token {:?} in expression", other))),
            None => Err(ParseError::new("Unexpected end of input")),
        }
    }

    fn parse_identifier_expression(&mut self, name: String) -> Result<ASTNode> {
        let mut expression = if name.starts_with('$') {
            ASTNode::Variable(name.clone())
        } else {
            ASTNode::Identifier(name.clone())
        };

        loop {
            match self.peek() {
                Some(Token::LParen) => {
                    expression = self.parse_call(expression)?;
                }
                Some(Token::LBrace) => {
                    let block = self.parse_block()?;
                    expression = ASTNode::Command {
                        name: name.clone(),
                        args: vec![block],
                    };
                }
                Some(Token::Dot) => {
                    self.next();
                    let property = self.expect_identifier()?;
                    expression = ASTNode::PropertyAccess {
                        target: Box::new(expression),
                        property,
                    };
                }
                _ => break,
            }
        }

        Ok(expression)
    }

    fn parse_call(&mut self, function: ASTNode) -> Result<ASTNode> {
        self.expect_token(Token::LParen)?;
        let mut args = Vec::new();

        while !matches!(self.peek(), Some(Token::RParen)) {
            args.push(self.parse_expression()?);
            if matches!(self.peek(), Some(Token::Comma)) {
                self.next();
            } else {
                break;
            }
        }

        self.expect_token(Token::RParen)?;

        let name = self.expression_to_command_name(&function)
            .ok_or_else(|| ParseError::new("Unsupported function call target"))?;

        Ok(ASTNode::Command { name, args })
    }

    fn expression_to_command_name(&self, expression: &ASTNode) -> Option<String> {
        match expression {
            ASTNode::Identifier(name) => Some(name.clone()),
            ASTNode::Variable(name) => Some(name.clone()),
            ASTNode::PropertyAccess { target, property } => {
                self.expression_to_command_name(target).map(|base| format!("{}.{property}", base))
            }
            _ => None,
        }
    }

    fn parse_block(&mut self) -> Result<ASTNode> {
        let mut body = Vec::new();
        self.expect_token(Token::LBrace)?;

        while !matches!(self.peek(), Some(Token::RBrace)) {
            body.push(self.parse_expression()?);
            if matches!(self.peek(), Some(Token::Semicolon)) {
                self.next();
            } else {
                break;
            }
        }

        self.expect_token(Token::RBrace)?;

        Ok(ASTNode::Block { body })
    }

    fn parse_number_literal(&self, raw: &str) -> Result<Literal> {
        let digits: String = raw.chars().take_while(|ch| ch.is_ascii_digit()).collect();
        let unit: String = raw.chars().skip_while(|ch| ch.is_ascii_digit()).collect();

        if digits.is_empty() {
            return Err(ParseError::new("Numeric literal must start with a digit"));
        }

        let value = digits.parse::<i64>().map_err(|_| ParseError::new("Failed to parse numeric literal"))?;
        let unit = if unit.is_empty() { None } else { Some(unit) };

        Ok(Literal::Number { value, unit })
    }

    fn peek_is_command_token(&self) -> bool {
        matches!(self.peek(),
            Some(Token::Identifier(_))
            | Some(Token::Word(_))
            | Some(Token::Flag(_))
            | Some(Token::StringLiteral(_))
            | Some(Token::Number(_))
            | Some(Token::Boolean(_))
            | Some(Token::LParen)
        )
    }

    fn peek_is_end_of_command(&self) -> bool {
        matches!(self.peek(), Some(Token::Pipe)
            | Some(Token::RParen)
            | Some(Token::RBrace)
            | Some(Token::Comma)
            | Some(Token::Semicolon)
            | Some(Token::GreaterThan)
            | Some(Token::AppendGreaterThan)
            | Some(Token::LessThan)
            | Some(Token::CombinedRedirect)
            | Some(Token::Fd(_)))
    }

    fn parse_command_from_name(&mut self, name: String) -> Result<ASTNode> {
        let mut args = Vec::new();

        while let Some(token) = self.peek().cloned() {
            match token {
                Token::Word(text)
                | Token::Identifier(text)
                | Token::Flag(text)
                | Token::StringLiteral(text) => {
                    self.next();
                    args.push(ASTNode::Literal(Literal::String(text)));
                }
                Token::Number(text) => {
                    self.next();
                    args.push(ASTNode::Literal(self.parse_number_literal(&text)?));
                }
                Token::Boolean(value) => {
                    self.next();
                    args.push(ASTNode::Literal(Literal::Boolean(value)));
                }
                Token::Fd(_) => break,
                _ if self.peek_redirect_start() => break,
                _ if matches!(token, Token::Pipe | Token::RParen | Token::RBrace | Token::Comma | Token::Semicolon) => break,
                _ => break,
            }
        }

        let mut command = ASTNode::Command { name, args };
        while self.peek_redirect_start() {
            command = self.parse_redirect(command)?;
        }
        Ok(command)
    }

    fn peek_redirect_start(&self) -> bool {
        matches!(self.peek(), Some(Token::GreaterThan)
            | Some(Token::AppendGreaterThan)
            | Some(Token::LessThan)
            | Some(Token::CombinedRedirect)
            | Some(Token::Fd(_)))
    }

    fn parse_redirect(&mut self, source: ASTNode) -> Result<ASTNode> {
        let mut fd = 1;
        let mut merge_stderr = false;

        if let Some(Token::Fd(fd_value)) = self.peek() {
            fd = *fd_value;
            self.next();
        }

        let mode = match self.next() {
            Some(Token::GreaterThan) => RedirectMode::Truncate,
            Some(Token::AppendGreaterThan) => RedirectMode::Append,
            Some(Token::LessThan) => {
                fd = 0;
                RedirectMode::Truncate
            }
            Some(Token::CombinedRedirect) => {
                fd = 1;
                merge_stderr = true;
                RedirectMode::Truncate
            }
            other => return Err(ParseError::new(format!("Expected redirect operator, found {:?}", other))),
        };

        let target = self.parse_redirect_target()?;
        Ok(ASTNode::Redirect {
            source: Box::new(source),
            fd,
            mode,
            target,
            merge_stderr,
        })
    }

    fn parse_redirect_target(&mut self) -> Result<RedirectTarget> {
        if let Some(Token::StringLiteral(text)) = self.peek() {
            let text = text.clone();
            self.next();
            return Ok(RedirectTarget::File(text));
        }

        let mut path = String::new();
        let mut consumed_any = false;

        while let Some(token) = self.peek().cloned() {
            match token {
                Token::Identifier(text) | Token::Word(text) | Token::Number(text) => {
                    path.push_str(&text);
                    self.next();
                    consumed_any = true;
                }
                Token::Dot => {
                    path.push('.');
                    self.next();
                    consumed_any = true;
                }
                Token::Slash => {
                    path.push('/');
                    self.next();
                    consumed_any = true;
                }
                Token::Flag(text) if consumed_any => {
                    path.push_str(&text);
                    self.next();
                }
                _ => break,
            }
        }

        if !consumed_any {
            return Err(ParseError::new("Expected redirect target, found end of input"));
        }

        Ok(RedirectTarget::File(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_basic_lets_and_pipeline() {
        let tokens = tokenize_line("let x = 5").unwrap();
        assert_eq!(tokens, vec![Token::Let, Token::Identifier("x".into()), Token::Equals, Token::Number("5".into())]);

        let pipeline = tokenize_line("ls() | filter { $it.size > 1mb }").unwrap();
        assert!(pipeline.contains(&Token::Pipe));
    }

    #[test]
    fn parse_let_statement() {
        let ast = parse_line("let x = 5").unwrap();
        assert_eq!(
            ast,
            ASTNode::Let {
                name: "x".into(),
                value: Box::new(ASTNode::Literal(Literal::Number { value: 5, unit: None })),
            }
        );
    }

    #[test]
    fn parse_pipeline_with_function_block() {
        let ast = parse_line("ls() | filter { $it.size > 1mb }").unwrap();

        assert!(matches!(ast, ASTNode::Pipeline { .. }));
    }

    #[test]
    fn parse_command_with_output_redirect() {
        let ast = parse_line("echo hello > out.txt").unwrap();

        assert_eq!(
            ast,
            ASTNode::Redirect {
                source: Box::new(ASTNode::Command {
                    name: "echo".into(),
                    args: vec![ASTNode::Literal(Literal::String("hello".into()))],
                }),
                fd: 1,
                mode: RedirectMode::Truncate,
                target: RedirectTarget::File("out.txt".into()),
                merge_stderr: false,
            }
        );
    }

    #[test]
    fn parse_pipeline_with_redirected_stage() {
        let ast = parse_line("echo hello | cat > out.txt").unwrap();

        assert!(matches!(ast, ASTNode::Pipeline { ref stages } if stages.len() == 2));
        if let ASTNode::Pipeline { stages } = ast {
            assert!(matches!(*stages[1], ASTNode::Redirect { .. }));
        }
    }
}
