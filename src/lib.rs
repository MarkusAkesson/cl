extern crate fastchr;
extern crate memmap;

use std::path::Path;
use std::fs::File;
use std::cmp::{max, min};
use std::fmt;

use fastchr::fastchr;
use memmap::{Mmap};

#[derive(PartialEq, Default, Clone)]
pub struct Count {
    pub code: u32,
    pub blank: u32,
    pub comments: u32,
    pub total: u32,
}

impl Count {
    pub fn merge(&mut self, count2: &Count) {
        self.code += count2.code;
        self.blank += count2.blank;
        self.comments += count2.comments;
        self.total += count2.total;
    }
}

pub struct CountTotal {
    pub files: u32,
    pub count: Count,
}

pub enum LineConfig<'a> {
    Normal {
        single: Option<&'a str>,
        multi: Option<(&'a str, &'a str)>,
    },
    Everything {
        singles: Vec<&'a str>,
        multis: Vec<(&'a str, &'a str)>,
    },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Clone)]
pub enum Language {
    C,
    Cpp,
    Header,
    Python,
    Rust,
    Java,
    Javascript,
    Go,
    Html,
    Makefile,
    Unknown,
    Assembly,
    Text,
    Markdown,
}
use self::Language::*;

impl language {
    pub fn to_string(&self) -> &str{
        match *self {
            C => "C",
            Header => "Header",
            Cpp => "C++",
            Python => "Python",
            Rust => "Rust",
            Java => "Java",
            Javascript => "Javascript",
            Go => "Go",
            Html => "Html",
            Makefile => "Makefile",
            Unknown => "Unknown",
            Assembly => "Assembly",
            Text => "Plain text",
            Markdown => "Markdown",

        }
    }
}


impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(self.to_string())
    }
}


pub fn get_language(filepath: &str) -> Language {
    let path = Path::new(filepath);

    let filename_lower = path.file_name()
        .expect("no filename")
        .to_str()
        .expect("to string")
        .to_lowercase();
    let ext = if filename_lower.contains("makefile") {
        String::from("makefile")
    } else {
        match path.extension() {
            Some(os_str) => os_str.to_str().unwrap().to_lowercase(),
            None => filename_lower,
        }
    };

    match &*ext {
        "c" => C,
        "cpp" | "cc" => Cpp,
        "h" | "hh" | "hpp" => Header,
        "py" => Python,
        "rs" => Rust,
        "java" => Java,
        "js" => Javascript,
        "go" => Go,
        "html" => Html,
        "makefile" => Makefile,
        "s" | "asm" => Assembly,
        "txt" => Text,
        "md" => Markdown,
        _ => Unknown,
    }
}

enum CommentStyle<'a> {
    Simple(Option<&'a str>, Option<(&'a str,&'a str)>),
    Extensive(Vec<&'a str>, Vec<(&'a str,&'a str)>),
}

use self::CommentStyle::*;

fn get_language_config<'a>(language: &Language) ->LineConfig <'a> {
    let c_style = Simple(Some("//"), Some(("/*", "*/")));
    let html_style = Simple(None, Some(("<!--", "-->")));
    let no_comments = Simple(None, None);
    let sh_style = Simple(Some("#"), None);

    let comment_style = match *language {
        C | Cpp | Rust | Java | Go | Javascript | Header => c_style,
        Assembly => Simple(Some("#"), Some(("/*", "*/"))),
        Python => Simple(Some("#"), Some(("'''", "'''"))),
        Text | Markdown | Unknown => no_comments,
        Makefile => sh_style,
        Html => html_style,
    };

    match comment_style {
        Simple(single,multi) => LineConfig::Normal {
            single,
            multi
        },
        Extensive(singles,multis) => LineConfig::Everything {
            singles,
            multis
        },
    }
}

struct ByteLinesState<'a> {
    buf: &'a [u8],
    pos: usize,
}

struct ByteLines<'a>(&'a [u8]);

impl <'a> ByteLines <'a> {
    fn lines(&self)-> ByteLinesState {
        ByteLinesState {
            buf: self.0,
            pos: 0,
        }
    }
}

impl<'a> Iterator for ByteLinesState<'a> {
    type Item = &'a [u8];
    fn next (&mut self) -> Option<&'a [u8]> {
        match fastchr(b'\n', &self.buf[self.pos..self.buf.len()]) {
            Some(n) => {
                let start = self.pos;
                self.pos = self.pos + n + 1; // add 1 to move pos to next line
                Some(&self.buf[start..(self.pos -1)]) // -1 to not pass \n as part of the line
            }
            None => {
                if self.pos == self.buf.len() {
                    return None;
                }
                let start = self.pos;
                self.pos = self.buf.len();
                Some(&self.buf[start..self.pos])
            }
        }
    }
}


pub fn count(filepath :&str) -> Count {
    let language = get_language(filepath);
    let cfg = get_language_config(&language);
    match cfg {
        LineConfig::Normal {single, multi} => count_normal(filepath,single,multi),
        LineConfig::Everything { singles, multis } => {
            count_everything(filepath,&singles,&multis)
        }
    }
}


pub fn count_normal(filepath: &str, single: Option<&str>, multi: Option<(&str,&str)>) -> Count {

    let file = File::open(filepath);
    let file = match file {
        Ok(file) => file,
        Err(_) => {
            return Count::default();
        },
    };

    let fmmap = unsafe {
        match Mmap::map(&file) {
            Ok(mmap) => mmap,
            Err(_) => {
                return Count::default();
            },
        }
    };

    let bytes: &[u8] = &fmmap;

    let mut count = Count::default();
    let mut in_comment = false;

    for byte_line in ByteLines(bytes).lines() {
        let line = match std::str::from_utf8(byte_line) {
            Ok(s) => s,
            Err(_) => return Count::default(),
        };

        count.total += 1;

        let line = line.trim();
        if line.is_empty() {
            count.blank += 1;
            continue;
        }


        if let Some(single) = single {
            if !in_comment && line.starts_with(single) {
                if let Some((multi_start, _)) = multi {
                    if !line.starts_with(multi_start) {
                        count.comments += 1;
                        continue;
                    }
                } else {
                    count.comments += 1;
                    continue;
                }
            }
        }

        let (multi_start,multi_end) = match multi {
            Some(multi) => multi,
            None => {
                count.code += 1; // Skip checking for comments if multiline commes are None
                continue;
            },
        };

        if !(line.contains(multi_start) || line.contains(multi_end)) {
            if in_comment {
                count.comments += 1;
            } else {
                count.code += 1;
            }
            continue;
        }


        let start_len = multi_start.len();
        let end_len = multi_end.len();
        let line_len = line.len();

        let mut pos = 0;
        let mut found_code = false;
        let contains_utf8 = (0..line_len).any(|i| !line.is_char_boundary(i));


        'outer: while pos < line_len {
            if contains_utf8 {
                for i in pos..pos + min(max(start_len, end_len) + 1, line_len - pos) {
                    if !line.is_char_boundary(i) {
                        pos += 1;
                        continue 'outer;
                    }
                }
            }

            if !in_comment && pos + start_len <= line_len
                && &line[pos..pos + start_len] == multi_start
            {
                pos += start_len;
                in_comment = true;
            } else if in_comment && pos + end_len <= line_len
                && &line[pos..pos + end_len] == multi_end
            {
                pos += end_len;
                in_comment = false;
            } else if !in_comment && !&line[pos..pos + 1].chars().next().unwrap().is_whitespace() {
                pos += 1;
                found_code = true;
            } else {
                pos += 1;
            }
        }

        if found_code {
            count.code += 1;
        } else {
            count.comments += 1;
        }
    }

    count
}


pub fn count_everything<'a>(filepath: &str, singles: &[&'a str], multis: &[(&'a str, &'a str)]) -> Count {
    let mut total_count = Count::default();
    for single in singles.iter() {
        let count = count_normal(filepath,Some(single),None);
        total_count.comments += count.comments;
        total_count.code -= count.comments;
    }
    for &(multi_start,multi_end) in multis {
        let count = count_normal(filepath,None,Some((multi_start,multi_end)));
        total_count.comments += count.comments;
        total_count.code -= count.comments;
    }
    total_count
}
