// Copyright © 2026 John P. ARNZEN
// SPDX-License-Identifier: MIT

use std::{iter::Peekable, str::Chars};

use crate::CliColor;
use color_lib::{Color, OKLAB, OKLCH, OKLrAB, OKLrCH, SRGB, Vector3};

#[derive(Debug)]
pub(crate) enum LexParseError {
    LexUnknownChar(char),
    LexInvalidNumber(std::num::ParseFloatError),
    LexEmptyIdentifier,
    LexUnexpectedChar(char),
    LexUnexpectedEndOfInput,
    ParseUnexpectedToken { expected: &'static str },
    ParseUnknownFunction(String),
    ParseUnknownColorSpace(String),
    ParseUnexpectedEndOfInput { expected: &'static str },
    ParseTrailingTokens,
}

impl std::error::Error for LexParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::LexInvalidNumber(e) => Some(e),
            _ => None,
        }
    }
}

impl std::fmt::Display for LexParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LexUnknownChar(c) => write!(f, "unknown character: {c:?}"),
            Self::LexInvalidNumber(_) => write!(f, "invalid number"),
            Self::LexEmptyIdentifier => write!(f, "empty identifier"),
            Self::LexUnexpectedChar(c) => write!(f, "unexpected character: {c:?}"),
            Self::LexUnexpectedEndOfInput => write!(f, "unexpected end of input"),
            Self::ParseUnexpectedToken { expected } => {
                write!(f, "unexpected token; expected: {expected}")
            }
            Self::ParseUnknownFunction(s) => write!(f, "unknown function: {s}"),
            Self::ParseUnknownColorSpace(s) => write!(f, "unknown color space: {s}"),
            Self::ParseUnexpectedEndOfInput { expected } => {
                write!(f, "unexpected end of input; expected: {expected}")
            }
            Self::ParseTrailingTokens => write!(f, "trailing tokens after function call"),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Ident(String),
    Number(f64),
    Percent,
    LParen,
    RParen,
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

    fn next_token(&mut self) -> Result<Option<Token>, LexParseError> {
        // skip whitespace and commas in all functions--diverges from CSS
        while self
            .chars
            .next_if(|c| c.is_whitespace() || *c == ',')
            .is_some()
        {}

        let Some(lexeme) = self.chars.peek().copied() else {
            return Ok(None);
        };

        match lexeme {
            '(' => {
                self.chars.next();
                Ok(Some(Token::LParen))
            }
            ')' => {
                self.chars.next();
                Ok(Some(Token::RParen))
            }
            '%' => {
                self.chars.next();
                Ok(Some(Token::Percent))
            }
            '-' => self.lex_dash().map(Some),
            c if c.is_ascii_digit() || c == '.' => {
                self.lex_number().map(|n| Some(Token::Number(n)))
            }
            c if c.is_ascii_alphabetic() => self.lex_ident().map(|s| Some(Token::Ident(s))),
            c => Err(LexParseError::LexUnknownChar(c)),
        }
    }

    fn lex_number(&mut self) -> Result<f64, LexParseError> {
        let mut buf = String::new();

        while let Some(c) = self.chars.next_if(|c| c.is_ascii_digit() || *c == '.') {
            buf.push(c);
        }

        buf.parse::<f64>().map_err(LexParseError::LexInvalidNumber)
    }

    fn lex_ident(&mut self) -> Result<String, LexParseError> {
        let mut result = String::new();

        while let Some(c) = self
            .chars
            .next_if(|c| c.is_ascii_alphanumeric() || *c == '-')
        {
            result.push(c);
        }

        if result.is_empty() {
            Err(LexParseError::LexEmptyIdentifier)
        } else {
            Ok(result)
        }
    }

    fn lex_dash(&mut self) -> Result<Token, LexParseError> {
        // consume the triggering dash
        self.chars.next();

        // peek what comes after the dash
        match self.chars.peek().copied() {
            // a custom ident like --ident
            Some('-') => {
                // consume this dash and get the identifier
                self.chars.next();
                let name = self.lex_ident()?;
                // add the dashes back for custom idents
                Ok(Token::Ident(format!("--{name}")))
            }
            // a negative number val
            Some(c) if c.is_ascii_digit() || c == '.' => {
                let num = self.lex_number()?;
                // negate the value
                Ok(Token::Number(-num))
            }
            // nothing else allowed
            Some(c) => Err(LexParseError::LexUnexpectedChar(c)),
            None => Err(LexParseError::LexUnexpectedEndOfInput),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, LexParseError>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.next_token() {
            Ok(Some(token)) => Some(Ok(token)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

struct Parser<'t> {
    tokens: &'t [Token],
    pos: usize,
}

impl<'t> Parser<'t> {
    fn new(tokens: &'t [Token]) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    // Parse CSS-style color function syntax into a CliColor.
    // Accepts color coordinates only, no alpha channel.
    //
    // color_func_spec: legacy_rgb_func | ok_func | color_func
    //
    // legacy_rgb_func: ID("rgb") '(' legacy_rgb_coord{3} ')'
    // ok_func: ID("oklch" | "oklab") '(' color_coord{3} ')'
    // color_func: ID("color") '(' color_space color_coord{3} ')'
    //
    // legacy_rgb_coord: percent_value | NUM
    // percent_value: NUM '%'
    // color_coord: NUM
    //
    // color_space: ID ("srgb" | "--oklch" | "--oklab" | "--oklrch" | "--oklrab")
    fn parse_color(&mut self) -> Result<CliColor, LexParseError> {
        // parse general form: ID '(' ID? NUM NUM NUM ')'
        let func_name = self.expect_ident("a function name")?;

        self.expect_symbol(&Token::LParen, "'('")?;

        let color = match func_name.as_str() {
            "rgb" => {
                let mut coords = [0.; 3];
                for c in &mut coords {
                    *c = self.parse_rgb_value()?;
                }
                CliColor::SRGB(Color::<SRGB>::new(coords))
            }
            "oklch" => CliColor::OKLCH(Color::<OKLCH>::new(self.parse_coords()?)),
            "oklab" => CliColor::OKLAB(Color::<OKLAB>::new(self.parse_coords()?)),
            "color" => {
                let color_space = self.expect_ident("a color space name")?;

                let make_color: fn(Vector3) -> CliColor = match color_space.as_str() {
                    "srgb" => |v| CliColor::SRGB(Color::<SRGB>::new(v)),
                    "--oklch" => |v| CliColor::OKLCH(Color::<OKLCH>::new(v)),
                    "--oklab" => |v| CliColor::OKLAB(Color::<OKLAB>::new(v)),
                    "--oklrch" => |v| CliColor::OKLrCH(Color::<OKLrCH>::new(v)),
                    "--oklrab" => |v| CliColor::OKLrAB(Color::<OKLrAB>::new(v)),
                    _ => return Err(LexParseError::ParseUnknownColorSpace(color_space)),
                };
                make_color(self.parse_coords()?)
            }
            _ => return Err(LexParseError::ParseUnknownFunction(func_name)),
        };

        self.expect_symbol(&Token::RParen, "')'")?;

        // if there's anything after the right paren, that's a parse error
        if self.peek().is_some() {
            return Err(LexParseError::ParseTrailingTokens);
        }

        Ok(color)
    }

    fn parse_rgb_value(&mut self) -> Result<f64, LexParseError> {
        let val = self.expect_number("a number")?;
        if self.peek() == Some(&Token::Percent) {
            self.advance();
            Ok(val / 100.)
        } else {
            Ok(val / 255.)
        }
    }

    fn parse_coords(&mut self) -> Result<Vector3, LexParseError> {
        let mut coords = [0.; 3];
        for c in &mut coords {
            *c = self.expect_number("a number")?;
        }
        Ok(coords)
    }

    fn expect_ident(&mut self, expected: &'static str) -> Result<String, LexParseError> {
        match self.advance() {
            Some(Token::Ident(ident)) => Ok(ident),
            Some(_) => Err(LexParseError::ParseUnexpectedToken { expected }),
            None => Err(LexParseError::ParseUnexpectedEndOfInput { expected }),
        }
    }

    fn expect_number(&mut self, expected: &'static str) -> Result<f64, LexParseError> {
        match self.advance() {
            Some(Token::Number(num)) => Ok(num),
            Some(_) => Err(LexParseError::ParseUnexpectedToken { expected }),
            None => Err(LexParseError::ParseUnexpectedEndOfInput { expected }),
        }
    }

    fn expect_symbol(&mut self, want: &Token, expected: &'static str) -> Result<(), LexParseError> {
        match self.advance() {
            Some(sym) if sym == *want => Ok(()),
            Some(_) => Err(LexParseError::ParseUnexpectedToken { expected }),
            None => Err(LexParseError::ParseUnexpectedEndOfInput { expected }),
        }
    }
}

pub(crate) fn parse_color_string(color_string: &str) -> Result<CliColor, LexParseError> {
    Lexer::new(color_string)
        .collect::<Result<Vec<Token>, LexParseError>>()
        .and_then(|tokens| Parser::new(&tokens).parse_color())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_rgb_function() {
        let src = "rgb(100% 128 50%)";
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<Token>, LexParseError>>()
            .unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("rgb".into()),
                Token::LParen,
                Token::Number(100.),
                Token::Percent,
                Token::Number(128.),
                Token::Number(50.),
                Token::Percent,
                Token::RParen,
            ]
        )
    }

    #[test]
    fn test_lexer_color_srgb_function() {
        let src = "color(srgb 1.0 0.5 0.2)";
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<Token>, LexParseError>>()
            .unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("color".into()),
                Token::LParen,
                Token::Ident("srgb".into()),
                Token::Number(1.),
                Token::Number(0.5),
                Token::Number(0.2),
                Token::RParen,
            ]
        )
    }

    #[test]
    fn test_lexer_color_oklch_function() {
        let src = "color(--oklch .99 .2 20)";
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<Token>, LexParseError>>()
            .unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("color".into()),
                Token::LParen,
                Token::Ident("--oklch".into()),
                Token::Number(0.99),
                Token::Number(0.2),
                Token::Number(20.),
                Token::RParen,
            ]
        )
    }

    #[test]
    fn test_lexer_oklab_with_negative_value() {
        let src = "oklab(.80 .2 -.3)";
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<Token>, LexParseError>>()
            .unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("oklab".into()),
                Token::LParen,
                Token::Number(0.8),
                Token::Number(0.2),
                Token::Number(-0.3),
                Token::RParen,
            ]
        )
    }

    #[test]
    fn test_lexer_rgb_with_commas() {
        let src = "rgb(10, 20, 30)";
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<Token>, LexParseError>>()
            .unwrap();
        assert_eq!(
            tokens,
            vec![
                Token::Ident("rgb".into()),
                Token::LParen,
                Token::Number(10.),
                Token::Number(20.),
                Token::Number(30.),
                Token::RParen,
            ]
        )
    }

    #[test]
    fn test_parser_rgb_func() {
        let src = "rgb(10% 128 10)";
        let color = parse_color_string(src).expect("color string should be parseable");
        let CliColor::SRGB(srgb_color) = color else {
            panic!("Expected an sRGB color; got something else.");
        };
        let expected = [0.1, 128. / 255., 10. / 255.];
        assert_eq!(srgb_color.coords, expected);
    }

    #[test]
    fn test_parser_color_srgb() {
        let src = "color(srgb 1 .5 .25)";
        let color = parse_color_string(src).expect("color string should be parseable");
        let CliColor::SRGB(srgb_color) = color else {
            panic!("Expected an sRGB color; got something else.");
        };
        let expected = [1., 0.5, 0.25];
        assert_eq!(srgb_color.coords, expected);
    }

    #[test]
    fn test_parser_oklab_func() {
        let src = "oklab(0.5 0.2 -0.2)";
        let color = parse_color_string(src).expect("color string should be parseable");
        let CliColor::OKLAB(oklab_color) = color else {
            panic!("Expected an OKLAB color; got something else.");
        };
        let expected = [0.5, 0.2, -0.2];
        assert_eq!(oklab_color.coords, expected);
    }

    #[test]
    fn test_parser_oklch_func() {
        let src = "oklch(0.5 0.4 110)";
        let color = parse_color_string(src).expect("color string should be parseable");
        let CliColor::OKLCH(oklch_color) = color else {
            panic!("Expected an OKLCH color; got something else.");
        };
        let expected = [0.5, 0.4, 110.];
        assert_eq!(oklch_color.coords, expected);
    }

    #[test]
    fn test_parser_color_oklrab() {
        let src = "color(--oklrab 0.05 0.2 -0.2)";
        let color = parse_color_string(src).expect("color string should be parseable");
        let CliColor::OKLrAB(lrab_color) = color else {
            panic!("Expected an OKLrAB color; got something else.");
        };
        let expected = [0.05, 0.2, -0.2];
        assert_eq!(lrab_color.coords, expected);
    }

    #[test]
    fn test_parser_color_oklrch() {
        let src = "color(--oklrch 0.05 0.4 110)";
        let color = parse_color_string(src).expect("color string should be parseable");
        let CliColor::OKLrCH(lrch_color) = color else {
            panic!("Expected an OKLrCH color; got something else.");
        };
        let expected = [0.05, 0.4, 110.];
        assert_eq!(lrch_color.coords, expected);
    }

    #[test]
    fn test_parser_color_oklab() {
        let src = "color(--oklab .5 .2 -.2)";
        let color = parse_color_string(src).expect("color string should be parseable");
        let CliColor::OKLAB(oklab_color) = color else {
            panic!("Expected an OKLAB color; got something else.");
        };
        let expected = [0.5, 0.2, -0.2];
        assert_eq!(oklab_color.coords, expected);
    }

    #[test]
    fn test_parser_color_oklch() {
        let src = "color(--oklch .5 .4 110)";
        let color = parse_color_string(src).expect("color string should be parseable");
        let CliColor::OKLCH(oklch_color) = color else {
            panic!("Expected an OKLCH color; got something else.");
        };
        let expected = [0.5, 0.4, 110.];
        assert_eq!(oklch_color.coords, expected);
    }

    // from Claude Code when asked about ensuring good code coverage
    #[test]
    fn test_invalid_inputs_are_rejected() {
        type ErrorPredicate = fn(&LexParseError) -> bool;
        let invalid_inputs: [(&str, ErrorPredicate); 12] = [
            ("oklch(0.5 0.2 @)", |e| {
                matches!(e, LexParseError::LexUnknownChar('@'))
            }),
            ("oklch(0.5 0.2 -x)", |e| {
                matches!(e, LexParseError::LexUnexpectedChar('x'))
            }),
            ("oklch(0.5 1.2.3 110)", |e| {
                matches!(e, LexParseError::LexInvalidNumber(_))
            }),
            ("oklch(0.5 0.2 -", |e| {
                matches!(e, LexParseError::LexUnexpectedEndOfInput)
            }),
            ("color(-- 1 2 3)", |e| {
                matches!(e, LexParseError::LexEmptyIdentifier)
            }),
            (
                "hsl(1 2 3)",
                |e| matches!(e, LexParseError::ParseUnknownFunction(s) if s == "hsl"),
            ),
            (
                "color(foo 1 2 3)",
                |e| matches!(e, LexParseError::ParseUnknownColorSpace(s) if s == "foo"),
            ),
            ("oklch(0.5 0.2)", |e| {
                matches!(
                    e,
                    LexParseError::ParseUnexpectedToken {
                        expected: "a number"
                    }
                )
            }),
            ("oklch(0.5 0.2 110", |e| {
                matches!(
                    e,
                    LexParseError::ParseUnexpectedEndOfInput { expected: "')'" }
                )
            }),
            ("oklch(0.5 0.2 110) junk", |e| {
                matches!(e, LexParseError::ParseTrailingTokens)
            }),
            ("", |e| {
                matches!(
                    e,
                    LexParseError::ParseUnexpectedEndOfInput {
                        expected: "a function name"
                    }
                )
            }),
            ("color(srgb 50% 0.5 0.2)", |e| {
                matches!(
                    e,
                    LexParseError::ParseUnexpectedToken {
                        expected: "a number"
                    }
                )
            }),
        ];

        let mut mismatched_errors = vec![];
        for (test_str, expect_pred) in invalid_inputs {
            let Err(err_result) = parse_color_string(test_str) else {
                panic!("test string {test_str:?} passed when it shouldn't have.")
            };
            if !expect_pred(&err_result) {
                mismatched_errors.push((test_str, err_result));
            }
        }
        assert!(
            mismatched_errors.is_empty(),
            "tests failed with wrong error:\n{:#?}",
            mismatched_errors
        )
    }
}
