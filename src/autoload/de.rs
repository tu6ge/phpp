use std::{fs::create_dir_all, path::Path, str::CharIndices};

use crate::error::ComposerError;

use super::{FilesData, Psr4Data};

impl Psr4Data {
    pub fn new() -> Result<Self, ComposerError> {
        let path = Path::new("./vendor/composer/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("autoload_psr4.php");
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => return Err(e.into()),
        };

        Ok(Self::parse(&content))
    }

    pub(crate) fn parse(str: &str) -> Self {
        let mut cursor = Cursor::new(str);

        let mut tokens = Vec::new();
        loop {
            let token = cursor.advance();
            match token {
                Some(Token::Other) | Some(Token::Space) | Some(Token::Dot) => {
                    continue;
                }
                Some(t) => tokens.push(t),
                None => break,
            }
        }
        let mut this = Self::default();
        //println!("{:?}", tokens);

        let mut iter = tokens.iter();
        let mut vendor_key = String::new();
        let mut is_vendor = false;
        loop {
            let token = iter.next();
            match token {
                Some(Token::Literal(str)) => {
                    if vendor_key.is_empty() {
                        vendor_key = str.to_owned().replace("\\\\", "\\");
                    } else {
                        this.data
                            .entry(vendor_key.clone())
                            .and_modify(|v| v.push((is_vendor, str.to_owned())))
                            .or_insert(vec![(is_vendor, str.to_owned())]);
                    }
                }
                Some(Token::VendorDir) => {
                    is_vendor = true;
                }
                Some(Token::BaseDir) => {
                    is_vendor = false;
                }
                Some(Token::ArrayEnd) => {
                    if !vendor_key.is_empty() {
                        vendor_key = "".to_owned();
                    }
                    is_vendor = false;
                }
                None => break,
                _ => {}
            }
        }

        this
    }
}
impl FilesData {
    pub fn new() -> Result<Self, ComposerError> {
        let path = Path::new("./vendor/composer/");
        if !path.exists() {
            create_dir_all(path)?;
        }
        let path = path.join("autoload_files.php");
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => return Err(e.into()),
        };
        Ok(Self::parse(&content))
    }

    pub(crate) fn parse(str: &str) -> Self {
        let mut cursor = Cursor::new(str);

        let mut tokens = Vec::new();
        loop {
            let token = cursor.advance();
            match token {
                Some(Token::Other) | Some(Token::Space) | Some(Token::Dot) => {
                    continue;
                }
                Some(t) => tokens.push(t),
                None => break,
            }
        }
        let mut this = Self::default();
        //println!("{:?}", tokens);

        let mut iter = tokens.iter();
        let mut vendor_key = String::new();
        let mut is_vendor = false;
        loop {
            let token = iter.next();
            match token {
                Some(Token::Literal(str)) => {
                    if vendor_key.is_empty() {
                        vendor_key = str.to_owned();
                    } else {
                        this.data
                            .insert(vendor_key.to_string(), (is_vendor, str.to_owned()));
                    }
                }
                Some(Token::VendorDir) => {
                    is_vendor = true;
                }
                Some(Token::BaseDir) => {
                    is_vendor = false;
                }
                Some(Token::ArraySplit) => {
                    if !vendor_key.is_empty() {
                        vendor_key = "".to_owned();
                    }
                    is_vendor = false;
                }
                None => break,
                _ => {}
            }
        }

        this
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Return,
    Space,
    ArrayStart,
    ArrayEnd,
    ArraySplit,
    //Quot,
    Literal(String),
    VendorDir,
    BaseDir,

    Arrow,
    Dot,

    Other,
}

#[derive(Clone)]
pub struct Cursor<'a> {
    source_str: &'a str,
    char: CharIndices<'a>,
}
impl<'a> Cursor<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source_str: source,
            char: source.char_indices(),
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
                            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '\\' | '/' | '.' | '_',
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
                let (_, c) = iter.next()?;
                if c == 'v' {
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    Some(Token::VendorDir)
                } else if c == 'b' {
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    self.char.next()?;
                    Some(Token::BaseDir)
                } else {
                    Some(Token::Other)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_psr4() {
        //let content = include_str!("../../vendor/composer/autoload_psr4.php");

        let content = r#"return array(
        'voku\\' => array(
            $vendorDir . '/voku/portable-ascii/src/voku',
        ),
        'Webmozart\\Assert\\' => array(
            $baseDir . '/webmozart/assert/src',
            $vendorDir . '/webmozart/assert/src2',
        ),"#;
        //dbg!(content);
        let res = Psr4Data::parse(content);
        println!("{:#?}", res);
    }

    // #[test]
    // fn test_parse_files() {
    //     let content = include_str!("../../vendor/composer/autoload_files.php");
    //     let mut cursor = Cursor::new(content);

    //     let mut tokens = Vec::new();
    //     loop {
    //         let token = cursor.advance();
    //         match token {
    //             Some(Token::Other) | Some(Token::Space) | Some(Token::Dot) => {
    //                 continue;
    //             }
    //             Some(t) => tokens.push(t),
    //             None => break,
    //         }
    //     }
    //     println!("{:#?}", tokens);
    // }
    #[test]
    fn test_real_files_parse() {
        let files = FilesData::new().unwrap();
        println!("{:#?}", files);
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

        let mut cursor = Cursor::new("$baseDir  ");
        assert_eq!(cursor.advance(), Some(Token::BaseDir));
        assert_eq!(cursor.advance(), Some(Token::Space));
        assert_eq!(cursor.advance(), None);

        let mut cursor = Cursor::new("$vendorDir  return");
        assert_eq!(cursor.advance(), Some(Token::VendorDir));
        assert_eq!(cursor.advance(), Some(Token::Space));
        assert_eq!(cursor.advance(), Some(Token::Return));

        let mut cursor = Cursor::new("$vendorDir . '/voku/portable-ascii/src/voku'");
        assert_eq!(cursor.advance(), Some(Token::VendorDir));
        assert_eq!(cursor.advance(), Some(Token::Space));
        assert_eq!(cursor.advance(), Some(Token::Dot));
        assert_eq!(cursor.advance(), Some(Token::Space));
        assert_eq!(
            cursor.advance(),
            Some(Token::Literal("/voku/portable-ascii/src/voku".to_string()))
        );
    }
}
