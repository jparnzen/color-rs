// Copyright © 2026 John P. ARNZEN
// SPDX-License-Identifier: MIT

use std::{iter::Peekable, str::Chars};

use crate::{CliColor, CliError};
use color_lib::{Color, OKLAB, OKLCH, OKLrAB, OKLrCH, SRGB, Vector3};

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

    fn next_token(&mut self) -> Result<Option<Token>, CliError> {
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
            _ => Err(CliError::ParseError),
        }
    }

    fn lex_number(&mut self) -> Result<f64, CliError> {
        let mut buf = String::new();

        while let Some(c) = self.chars.next_if(|c| c.is_ascii_digit() || *c == '.') {
            buf.push(c);
        }

        buf.parse::<f64>().map_err(|_| CliError::ParseError)
    }

    fn lex_ident(&mut self) -> Result<String, CliError> {
        let mut result = String::new();

        while let Some(c) = self
            .chars
            .next_if(|c| c.is_ascii_alphanumeric() || *c == '-')
        {
            result.push(c);
        }

        if result.is_empty() {
            Err(CliError::ParseError)
        } else {
            Ok(result)
        }
    }

    fn lex_dash(&mut self) -> Result<Token, CliError> {
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
            _ => Err(CliError::ParseError),
        }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, CliError>;
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

    fn parse_color(&mut self) -> Result<CliColor, CliError> {
        // parse ID '(' ID? NUM NUM NUM ')'
        let Some(Token::Ident(func_name)) = self.advance() else {
            return Err(CliError::ParseError);
        };

        let Some(Token::LParen) = self.advance() else {
            return Err(CliError::ParseError);
        };

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
                let Some(Token::Ident(color_space)) = self.advance() else {
                    return Err(CliError::ParseError);
                };
                let coords = self.parse_coords()?;
                match color_space.as_str() {
                    "srgb" => CliColor::SRGB(Color::<SRGB>::new(coords)),
                    "--oklch" => CliColor::OKLCH(Color::<OKLCH>::new(coords)),
                    "--oklab" => CliColor::OKLAB(Color::<OKLAB>::new(coords)),
                    "--oklrch" => CliColor::OKLrCH(Color::<OKLrCH>::new(coords)),
                    "--oklrab" => CliColor::OKLrAB(Color::<OKLrAB>::new(coords)),
                    _ => return Err(CliError::ParseError),
                }
            }
            _ => return Err(CliError::ParseError),
        };

        let Some(Token::RParen) = self.advance() else {
            return Err(CliError::ParseError);
        };

        // if there's anything after the right paren, that's a parse error
        if self.peek().is_some() {
            return Err(CliError::ParseError);
        }

        Ok(color)
    }

    fn parse_coords(&mut self) -> Result<Vector3, CliError> {
        let mut coords = [0.; 3];
        for c in &mut coords {
            *c = self.parse_float_value()?;
        }
        Ok(coords)
    }

    fn parse_rgb_value(&mut self) -> Result<f64, CliError> {
        let Some(Token::Number(val)) = self.advance() else {
            return Err(CliError::ParseError);
        };
        if self.peek() == Some(&Token::Percent) {
            self.advance();
            Ok(val / 100.)
        } else {
            Ok(val / 255.)
        }
    }

    fn parse_float_value(&mut self) -> Result<f64, CliError> {
        let Some(Token::Number(val)) = self.advance() else {
            return Err(CliError::ParseError);
        };
        Ok(val)
    }
}

fn parse_tokens(tokens: &[Token]) -> Result<CliColor, CliError> {
    Parser::new(tokens).parse_color()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Acceptable color strings:
    // rgb(254, 100%, 50%)
    // rgb(100%, 90%, 80)
    // rgb(1 2 3)
    // oklch(.90 .2 20)
    // oklab(0.80 0.3 -.2)
    // color(srgb 0.9 .5 .2)
    // color(--oklch .90 .2 20)
    // color(--oklab .80 .3 -0.2)
    // color(--oklrch 0.02 .2 240.2)
    // color(--oklrab 0.01 .2 -0.3)

    #[test]
    fn test_lexer_rgb_function() {
        let src = "rgb(100% 128 50%)";
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<Token>, CliError>>()
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
            .collect::<Result<Vec<Token>, CliError>>()
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
            .collect::<Result<Vec<Token>, CliError>>()
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
            .collect::<Result<Vec<Token>, CliError>>()
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
            .collect::<Result<Vec<Token>, CliError>>()
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
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<Token>, CliError>>()
            .unwrap();
        let color = parse_tokens(&tokens).expect("expected parseable color string");
        let CliColor::SRGB(srgb_color) = color else {
            panic!("Expected an sRGB color; got something else.");
        };
        let expected = [0.1, 128. / 255., 10. / 255.];
        assert_eq!(srgb_color.coords, expected);
    }

    #[test]
    fn test_parser_color_srgb() {
        let src = "color(srgb 1 .5 .25)";
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<Token>, CliError>>()
            .unwrap();
        let color = parse_tokens(&tokens).expect("expected parseable color string");
        let CliColor::SRGB(srgb_color) = color else {
            panic!("Expected an sRGB color; got something else.");
        };
        let expected = [1., 0.5, 0.25];
        assert_eq!(srgb_color.coords, expected);
    }

    // from Claude Code when asking about good code coverage
    #[test]
    fn test_invalid_inputs_are_rejected() {
        let invalid_inputs = [
            "oklch(0.5 0.2 @)",
            "oklch(0.5 1.2.3 110)",
            "oklch(0.5 0.2 -",
            "color(-- 1 2 3)",
            "hsl(1 2 3)",
            "color(foo 1 2 3)",
            "oklch(0.5 0.2)",
            "oklch(0.5 0.2 110",
            "oklch(0.5 0.2 110) junk",
            "",
            "color(srgb 50% 0.5 0.2)",
        ];
        for input in invalid_inputs {
            let result = Lexer::new(input)
                .collect::<Result<Vec<Token>, CliError>>()
                .and_then(|tokens| parse_tokens(&tokens));
            assert!(result.is_err(), "expected an error for input: {input:?}");
        }
    }
}
