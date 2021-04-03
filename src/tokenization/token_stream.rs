use std::{collections::HashSet};
use std::collections::HashMap;
use std::iter::FromIterator;
use crate::position::{FilePosition, START_POSITION};
use crate::error::{CompilationError, CompilationErrorKind};
use crate::tokenization::{
    token::*,
    buffer::{Buffer}
};

type TokenizationResult = std::result::Result<Token, CompilationError>;

/// A stream of tokens
pub struct TokenStream<T: Buffer> {
    prev_pos: FilePosition,
    buffer: T,
    reserved_words: HashMap<String, Token>,
    lexeme_start: usize,
    state: i32
}

impl<T: Buffer> TokenStream<T> {
    /// Creates a new TokenStream based on the provided stream.
    pub fn new(buffer: T) -> TokenStream<T> {
        TokenStream {
            prev_pos: START_POSITION,
            buffer: buffer,
            state: 1,
            reserved_words: [
                ("program".to_string(), Token::K(Keyword::Program)),
                ("procedure".to_string(), Token::K(Keyword::Procedure)),
                ("div".to_string(), Token::O(Operator::IntegerDivide)),
                ("record".to_string(), Token::K(Keyword::Record)),
                ("xor".to_string(), Token::O(Operator::Xor)),
                ("or".to_string(), Token::O(Operator::Or)),
                ("and".to_string(), Token::O(Operator::And)),
                ("not".to_string(), Token::O(Operator::Not)),
                ("if".to_string(), Token::K(Keyword::If)),
                ("then".to_string(), Token::K(Keyword::Then)),
                ("else".to_string(), Token::K(Keyword::Else)),
                ("of".to_string(), Token::K(Keyword::Of)),
                ("while".to_string(), Token::K(Keyword::While)),
                ("do".to_string(), Token::K(Keyword::Do)),
                ("begin".to_string(), Token::K(Keyword::Begin)),
                ("end".to_string(), Token::K(Keyword::End)),
                ("var".to_string(), Token::K(Keyword::Var)),
                ("type".to_string(), Token::K(Keyword::Type)),
                ("array".to_string(), Token::K(Keyword::Array)),
                ("for".to_string(), Token::K(Keyword::For)),
                ("repeat".to_string(), Token::K(Keyword::Repeat)),
                ("with".to_string(), Token::K(Keyword::With)),
                ("until".to_string(), Token::K(Keyword::Until)),
                ("to".to_string(), Token::K(Keyword::To)),
                ("downto".to_string(), Token::K(Keyword::Downto))
            ].iter().cloned().collect(),
            lexeme_start: 0,
        }
    }

    pub fn filepath(&self) -> &Option<String> {
        self.buffer.file()
    }

    pub fn pos(&self) -> FilePosition {
        self.buffer.pos()
    }

    pub fn prev_pos(&self) -> FilePosition {
        self.buffer.prev_pos()
    }

    /// Reads a token from the `stream`.
    pub fn next(&mut self) -> TokenizationResult {
        loop {
            let pos = self.buffer.shift();
            let c = self.buffer.next().unwrap() as char;

            match self.state {
                1 => {
                    if c.is_whitespace() {
                        self.skip_whitespace();
                    } else if c == '{' {
                        self.skip_comment();
                    } else {
                        self.lexeme_start = pos;
                        if c.is_numeric() {
                            self.state = 4;
                        } else if c.is_alphabetic() {
                            self.state = 2;
                        } else {
                            match c {
                                '.' => self.state = 17,
                                ':' => self.state = 20,
                                '\'' => {
                                    self.lexeme_start = self.buffer.shift();
                                    self.state = 13;
                                },
                                '<' => {
                                    self.state = 23
                                },
                                '>' => self.state = 24,
                                '=' => return Ok(Token::R(Relation::Eq)),
                                '+' => return Ok(Token::O(Operator::Plus)),
                                '-' => return Ok(Token::O(Operator::Minus)),
                                '/' => return Ok(Token::O(Operator::Divide)),
                                '*' => return Ok(Token::O(Operator::Multiply)),
                                ',' => {
                                    self.state = 1;
                                    return Ok(
                                        Token::P(
                                            Punctuation::Comma
                                        )
                                    );
                                },
                                ';' => {
                                    self.state = 1;
                                    return Ok(
                                        Token::P(
                                            Punctuation::Semicolon
                                        )
                                    )
                                },
                                '(' => {
                                    self.state = 1;
                                    return Ok(
                                        Token::P(
                                            Punctuation::Lbracket
                                        )
                                    )
                                },
                                ')' => {
                                    self.state = 1;
                                    return Ok(
                                        Token::P(
                                            Punctuation::Rbracket
                                        )
                                    )
                                }
                                '[' => {
                                    self.state = 1;
                                    return Ok(
                                        Token::P(
                                            Punctuation::Lsqbracket
                                        ),
                                    )
                                },
                                ']' => {
                                    self.state = 1;
                                    return Ok(
                                        Token::P(
                                            Punctuation::Rsqbracket
                                        )
                                    )
                                },
                                '\0' => return Ok(Token::EOF),
                                _ => {
                                    self.state = 1;
                                    return Err(self.error(
                                        "Unexpected character"
                                    ))
                                }
                            }
                        }
                    }

                },
                2 => {
                    if !c.is_alphanumeric() && c != '_' {
                        self.buffer.back(1);
                        self.state = 1;
                        return Ok(self.identifier());
                    }
                },
                4 => {
                    if c == '.' {
                        self.state = 5;
                    } else if !c.is_numeric() {
                        self.buffer.back(1);
                        let number = self.number();
                        self.state = 1;
                        return Ok(number);
                    }
                },
                5 => {
                    if c.is_numeric() {
                        self.state = 6;   
                    }
                    else if c == '.' {
                        self.buffer.back(2);
                        let number = self.number();
                        self.state = 1;
                        return Ok(number);
                    }
                },
                6 => {
                    if c.to_ascii_lowercase() == 'e' {
                        self.state = 7;
                    } else if !c.is_numeric() {
                        self.buffer.back(1);
                        let number = self.number();
                        self.state = 1;
                        return Ok(number);
                    }
                },
                7 => {
                    if c.is_numeric() {
                        self.state = 9;
                    } else if c == '+' || c == '-' {
                        self.state = 8;
                    }
                },
                8 => {
                    if c.is_numeric() {
                        self.state = 9;
                    } else {
                        self.state = 1;
                        return Err(self.error(
                            "Sign in scientific notation \
                            must be followed by a number"
                        ))
                    }
                },
                9 => {
                    if !c.is_numeric() {
                        let number = self.number();
                        return Ok(number);
                    }
                }
                13 => {
                    if c == '\'' {
                        self.state = 1;
                        return Ok(self.literal());
                    } else if c == '\n' {
                        self.state = 1;
                        return Err(self.error(
                            "string literal cannot span multiple lines"
                        ))
                    }
                },
                17 => {
                    self.state = 1;
                    if c == '.' {
                        return Ok(Token::P(Punctuation::Range));
                    } else {
                        self.buffer.back(1);
                        return Ok(Token::P(Punctuation::Dot));
                    }
                },
                20 => {
                    self.state = 1;
                    if c == '=' {
                        return Ok(Token::O(Operator::Assign))
                    } else {
                        self.buffer.back(1);
                        return Ok(Token::P(Punctuation::Colon));
                    }
                }
                23 => {
                    self.state = 1;
                    if c == '>' {
                        return Ok(Token::R(Relation::Ne))
                    } else if c == '=' {
                        return Ok(Token::R(Relation::Le))
                    } else {
                        self.buffer.back(1);
                        return Ok(Token::R(Relation::Lt))
                    }
                },
                24 => {
                    self.state = 1;
                    if c == '=' {
                        return Ok(Token::R(Relation::Ge))
                    } else {
                        self.buffer.back(1);
                        return Ok(Token::R(Relation::Gt))
                    }
                },
                _ => { 
                    /* Should never happen */
                    self.state = 1;
                    return Err(self.error("unknown error"));
                },
            }
        }
    }

    /// Reports whether some `token` in `tokens`
    /// is present further in the stream.
    pub fn available(
        &mut self, tokens: &[Token]
    ) -> Result<bool, CompilationError> {

        let token_set: HashSet<Token> = HashSet::from_iter(
            tokens.iter().cloned()
        );

        self.buffer.save_pos();
        let result;
        loop {
            let token = self.next()?;

            if token == Token::EOF {
                if token_set.contains(&Token::EOF) {
                    result = true
                } else {
                    result = false;
                }
                break;
            }
    
            if token_set.contains(&token) {
                result = true;
                break;
            }
        }

        self.buffer.restore_pos();
        Ok(result)
    }

    fn skip_whitespace(&mut self) {
        loop {
            let c = self.buffer.next().unwrap() as char;
            if !c.is_whitespace() {
                self.buffer.back(1);
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        loop {
            let c = self.buffer.next().unwrap() as char;
            if c == '}' || c == '\0' {
                self.buffer.next().unwrap();
                break;
            }
        }
    }

    fn number(&self) -> Token {
        let lexeme = self.lexeme();
        Token::Number(lexeme)
    }

    fn identifier(&self) -> Token {
        let lexeme = self.lexeme();
        if self.reserved_words.contains_key(&lexeme) {
            self.reserved_words.get(&lexeme).unwrap().clone()
        } else {
            Token::Id(lexeme)
        }
    }

    fn literal(&self) -> Token {
        let lexeme = self.lexeme().strip_suffix('\'').unwrap().to_string();
        Token::Literal(lexeme)
    }

    fn lexeme(&self) -> String {
        let range = self.buffer.range(
            self.lexeme_start,
            self.buffer.shift()
        );
        String::from_utf8(range).unwrap().to_lowercase()
    }

    fn error(&self, msg: &str) -> CompilationError {
        CompilationError::new(
            CompilationErrorKind::LexicalError,
            &self.filepath(),
            self.buffer.prev_pos(),
            msg
        )
    }
}

#[cfg(test)]
mod token_stream_tests {
    use super::*;
    use crate::tokenization::{Token, Keyword, Operator, Punctuation, Relation};
    use crate::tokenization::SimpleBuffer;
    use crate::position::FilePosition;

    fn token_stream(input: &str) -> TokenStream<SimpleBuffer> {
        let b = SimpleBuffer::new(input.as_bytes(), None);
        TokenStream::new(b)
    }

    #[test]
    fn test_next_number() {
        let input = "5";
        let mut ts = token_stream(input);
        let five = ts.next().unwrap();
        match five {
            Token::Number(n) => { assert_eq!(n, "5") },
            _ => assert!(false)
        }
    }

    #[test]
    fn test_next_long_number() {
        let input = "123";
        let mut ts = token_stream(input);
        let onetwothree = ts.next().unwrap();
        match onetwothree {
            Token::Number(n) => { assert_eq!(n, "123") }
            _ => assert!(false)
        }
    }

    #[test]
    fn test_next_number_with_space_after() {
        let input = "1 13";
        let ts = token_stream(input);
        
        let expected_tokens = [
            Token::Number("1".to_string()),
            Token::Number("13".to_string())
        ];

        assert_token_sequence(&expected_tokens, ts);
    }

    #[test]
    fn test_next_number_and_range() {
        let input = "1..6";
        let mut ts = token_stream(input);
        let one = ts.next().unwrap();
        match one {
            Token::Number(n) => { assert_eq!(n, "1") }
            _ => { assert!(false) }
        }

        let range = ts.next().unwrap();
        match range {
            Token::P(Punctuation::Range) => { assert!(true) },
            _ => { assert!(false) }
        }

        let six = ts.next().unwrap();
        match six {
            Token::Number(n) => { assert_eq!(n, "6") },
            _ => { assert!(false) }
        }
    }

    #[test]
    fn test_next_numbers() {
        let numbers = [
            "1.64123",
            "1.10e+30 ",
            "1.13e-12",
            "1.10e120",
            "1.13E1",
        ];

        for num in numbers.iter() {
            let mut ts = token_stream(num);
            let token = ts.next().unwrap();
            match token {
                Token::Number(lexeme) =>
                    assert_eq!(lexeme, *num.to_lowercase()),
                _ => assert!(false)
            }
        }
    }

    #[test]
    fn test_next_identifiers() {
        let identifiers = [
            "hello",
            "i",
            "am",
            "confused_here"
        ];

        for identifier in identifiers.iter() {
            let mut ts = token_stream(identifier);
            let token = ts.next().unwrap();
            match token {
                Token::Id(lexeme) => assert_eq!(lexeme, *identifier),
                _ => assert!(false)
            }
        }
    }

    #[test]
    fn test_next_keywords() {
        let keywords = [
            ("program", Keyword::Program),
            ("begin", Keyword::Begin),
            ("end", Keyword::End),
            ("of", Keyword::Of),
            ("var", Keyword::Var),
        ];

        for keyword in keywords.iter() {
            let mut ts = token_stream(keyword.0);
            let token = ts.next().unwrap();
            match token {
                Token::K(lexeme) => assert_eq!(lexeme, keyword.1),
                _ => assert!(false)
            }
        }
    }
    
    #[test]
    fn test_next_whitespace() {
        let input = "    thing   other_thing   ";
        let mut ts = token_stream(input);

        let expected_tokens = [
            Token::Id("thing".to_string()),
            Token::Id("other_thing".to_string())
        ];

        for t in expected_tokens.iter() {
            assert_eq!(*t, ts.next().unwrap());
        }
    }

    #[test]
    fn test_next_comments() {
        let input = "{{This is a comment}} some_identifier";
        let mut ts = token_stream(input);
        
        match ts.next().unwrap() {
            Token::Id(lexeme) => assert_eq!(lexeme, "some_identifier"),
            _ => assert!(false)
        }
    }

    #[test]
    fn test_next_literal() {
        let input = "'some string'";
        let mut ts = token_stream(input);

        match ts.next().unwrap() {
            Token::Literal(lexeme) => assert_eq!(lexeme, "some string"),
            _ => assert!(false)
        }
    }

    #[test]
    fn test_next_punctuation() {
        let input = "()[],...";
        let ts = token_stream(input);

        let expected_tokens = [
            Token::P(Punctuation::Lbracket),
            Token::P(Punctuation::Rbracket),
            Token::P(Punctuation::Lsqbracket),
            Token::P(Punctuation::Rsqbracket),
            Token::P(Punctuation::Comma),
            Token::P(Punctuation::Range),
            Token::P(Punctuation::Dot),
        ];

        assert_token_sequence(&expected_tokens, ts);
    }

    #[test]
    fn test_next_relational_operator() {
        let input = "if b == 25 then begin";
        let ts = token_stream(input);

        let expected_tokens = [
            Token::K(Keyword::If),
            Token::Id("b".to_string()),
            Token::R(Relation::Eq),
            Token::R(Relation::Eq),
            Token::Number("25".to_string()),
            Token::K(Keyword::Then),
            Token::K(Keyword::Begin)
        ];

        assert_token_sequence(&expected_tokens, ts);
    }

    #[test]
    fn text_eof_after_dot() {
        let input = ".";
        let ts = token_stream(input);

        let expected_tokens = [
            Token::P(Punctuation::Dot),
            Token::EOF
        ];

        assert_token_sequence(&expected_tokens, ts);
    }

    #[test]
    fn test_next_record_field() {
        let input = "a.b";
        let ts = token_stream(input);

        let expected_tokens = [
            Token::Id("a".to_string()),
            Token::P(Punctuation::Dot),
            Token::Id("b".to_string())
        ];

        assert_token_sequence(&expected_tokens, ts)
    }

    #[test]
    fn test_next_relations() {
        let input = "<<=><>>==";
        let ts = token_stream(input);

        let expected_tokens = [
            Token::R(Relation::Lt),
            Token::R(Relation::Le),
            Token::R(Relation::Gt),
            Token::R(Relation::Ne),
            Token::R(Relation::Ge),
            Token::R(Relation::Eq)
        ];

        assert_token_sequence(&expected_tokens, ts);
    }

    #[test]
    fn test_next_id_after_begin(){
        let input = 
            " begin
              c := 'a';
            ";

        let expected_tokens = [
            Token::K(Keyword::Begin),
            Token::Id("c".to_string()),
            Token::O(Operator::Assign),
            Token::Literal("a".to_string()),
            Token::P(Punctuation::Semicolon)
        ];

        let ts = token_stream(input);
        assert_token_sequence(&expected_tokens, ts);
    }

    #[test]
    fn test_next_operators() {
        let input = "a+ 42 - c/d *e";
        let ts = token_stream(input);

        let expected_tokens = [
            Token::Id("a".to_string()),
            Token::O(Operator::Plus),
            Token::Number("42".to_string()),
            Token::O(Operator::Minus),
            Token::Id("c".to_string()),
            Token::O(Operator::Divide),
            Token::Id("d".to_string()),
            Token::O(Operator::Multiply),
            Token::Id("e".to_string())
        ];

        assert_token_sequence(&expected_tokens, ts);
    }

    #[test]
    fn test_next_error() {
        let input = "2.3e+heh";
        let mut ts = token_stream(input);

        let err = ts.next().unwrap_err();
        assert_eq!(err.pos(), FilePosition { line: 1, col: 6 });
    }

    #[test]
    fn test_next_second_line_error() {
        let input = "2.3\n2.3e+heh";
        let mut ts = token_stream(input);

        ts.next().unwrap();
        let err = ts.next().unwrap_err();
        assert_eq!(err.pos(), FilePosition { line: 2, col: 6 });
    }

    #[test]
    fn test_next_eof() {
        let input = "";
        let mut ts = token_stream(input);

        assert_eq!(ts.next().unwrap(), Token::EOF);
    }

    #[test]
    fn test_next_double_colon() {
        let input = ": :;";
        let ts = token_stream(input);

        let expected = [
            Token::P(Punctuation::Colon),
            Token::P(Punctuation::Colon),
            Token::P(Punctuation::Semicolon),
        ];

        assert_token_sequence(&expected, ts);
    }

    #[test]
    fn test_pos() {
        let input = "1\n3\n5 7 9";
        let mut ts = token_stream(input);
    
        assert_eq!(FilePosition::new(1, 1), ts.prev_pos());

        ts.next().unwrap();
        assert_eq!(FilePosition::new(1, 2), ts.prev_pos());

        ts.next().unwrap();
        assert_eq!(FilePosition::new(2, 2), ts.prev_pos());

        ts.next().unwrap();
        assert_eq!(FilePosition::new(3, 2), ts.prev_pos());
    }

    #[test]
    fn test_available() {
        let input = "1 2 3 4 5 6";
        let mut ts = token_stream(input);

        assert!(ts.available(&[Token::Number("5".to_string())]).unwrap());
        assert_eq!(Token::Number("1".to_string()), ts.next().unwrap());
    }

    #[test]
    fn test_available_eof() {
        let input = "1 2 3 4 5 6";
        let mut ts = token_stream(input);

        assert!(ts.available(&[Token::EOF]).unwrap());    
    }

    #[test]
    fn test_real_semicolon() {
        let input = "0.0;";
        let ts = token_stream(input);

        let expected = [
            Token::Number("0.0".to_string()),
            Token::P(Punctuation::Semicolon)
        ];

        assert_token_sequence(&expected, ts);
    }

    fn assert_token_sequence<T: Buffer>(
        expected: &[Token], mut ts: TokenStream<T>
    ) {
        for t in expected.iter() {
            assert_eq!(*t, ts.next().unwrap());
        }
    }
}
