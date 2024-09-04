use std::str::CharIndices;

use indexmap::IndexMap;

struct Psr4Data {
    data: IndexMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Return,
    Space,
    ArrayStart,
    ArrayEnd,
    ArraySplit,
    Var,
    //Quot,
    Literal(String),

    Arrow,
    Dot,

    Other,
}

#[derive(Clone)]
pub struct Cursor<'a> {
    source_str: &'a str,
    char: CharIndices<'a>,
    token: Vec<Token>,
}
impl<'a> Cursor<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source_str: source,
            char: source.char_indices(),
            token: Vec::new(),
        }
    }

    fn advance(&mut self) -> Option<Token> {
        let (start_usize, char) = self.char.next()?;
        match char {
            'r' => {
                let mut iter = self.char.clone();
                let (_, c) = iter.next()?;
                if c == 'e' {
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    Some(Token::Return)
                } else {
                    Some(Token::Other)
                }
            }
            ' ' => {
                let mut iter = self.char.clone();
                loop {
                    match iter.next() {
                        Some((_, ' ')) => {
                            self.char.next();
                        }
                        _ => return Some(Token::Space),
                    }
                }
            }
            'a' => {
                let mut iter = self.char.clone();
                let (_, c) = iter.next()?;
                if c == 'r' {
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    Some(Token::ArrayStart)
                } else {
                    Some(Token::Other)
                }
            }
            '\'' => {
                let mut iter = self.char.clone();
                let mut current_usize = start_usize;
                loop {
                    match iter.next() {
                        Some((
                            last_usize,
                            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '\\' | '/',
                        )) => {
                            current_usize = last_usize;
                            self.char.next();
                        }
                        Some((_, '\'')) => {
                            let name = &self.source_str[start_usize + 1..current_usize + 1];
                            let token = Token::Literal(name.to_string());
                            self.char.next();
                            //self.token.push(token.clone());
                            return Some(token);
                        }
                        _ => return None,
                    }
                }
            }
            '=' => {
                let mut iter = self.char.clone();
                let (_, c) = iter.next()?;
                if c == '>' {
                    self.char.next()?;
                    Some(Token::Arrow)
                } else {
                    Some(Token::Other)
                }
            }
            '$' => {
                let mut iter = self.char.clone();
                loop {
                    match iter.next() {
                        Some((_, 'a'..='z' | 'A'..='Z' | '0'..='9')) => {
                            self.char.next();
                        }
                        Some((_, ' ')) | Some((_, ')')) => {
                            return Some(Token::Var);
                        }
                        _ => return None,
                    }
                }
            }
            '.' => Some(Token::Dot),
            ',' => Some(Token::ArraySplit),
            ')' => Some(Token::ArrayEnd),
            _ => {
                let mut iter = self.char.clone();
                loop {
                    match iter.next() {
                        Some((_, 'r' | ' ' | 'a' | '\'' | '=' | '$' | '.' | ',')) => {
                            return Some(Token::Other);
                        }
                        Some((_, _)) => {
                            self.char.next();
                        }
                        _ => return Some(Token::Other),
                    }
                }
            }
        }
    }

    pub fn parse(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.advance();
            match token {
                Some(Token::Other) | Some(Token::Space) => {
                    continue;
                }
                Some(t) => tokens.push(t),
                None => break,
            }
        }
        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works_simple() {
        let content = include_str!("../../vendor/composer/autoload_psr4.php");
        // dbg!(content);
        let mut cursor = Cursor::new(content);
        let tokens = cursor.parse();

        println!("{:?}", tokens);
    }

    #[test]
    fn it_works() {
        let mut cursor = Cursor::new("return array(");
        assert_eq!(cursor.advance(), Some(Token::Return));
        assert_eq!(cursor.advance(), Some(Token::Space));
        assert_eq!(cursor.advance(), Some(Token::ArrayStart));
        assert_eq!(cursor.advance(), None);

        let mut cursor = Cursor::new("'aaa\\bbb/ccc'return");
        assert_eq!(
            cursor.advance(),
            Some(Token::Literal("aaa\\bbb/ccc".to_string()))
        );
        assert_eq!(cursor.advance(), Some(Token::Return));

        let mut cursor = Cursor::new("$var  ");
        assert_eq!(cursor.advance(), Some(Token::Var));
        assert_eq!(cursor.advance(), Some(Token::Space));
        assert_eq!(cursor.advance(), None);

        let mut cursor = Cursor::new("$var  return");
        assert_eq!(cursor.advance(), Some(Token::Var));
        assert_eq!(cursor.advance(), Some(Token::Space));
        assert_eq!(cursor.advance(), Some(Token::Return));

        let mut cursor = Cursor::new("$vendorDir . '/voku/portable-ascii/src/voku'");
        assert_eq!(cursor.advance(), Some(Token::Var));
        assert_eq!(cursor.advance(), Some(Token::Space));
        assert_eq!(cursor.advance(), Some(Token::Dot));
        assert_eq!(cursor.advance(), Some(Token::Space));
        assert_eq!(
            cursor.advance(),
            Some(Token::Literal("/voku/portable-ascii/src/voku".to_string()))
        );
    }
}
