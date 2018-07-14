use std::path::Path;
use std::fs::File;
use std::cmp::{max, min};
use std::fmt;

    pub struct count {
    pub code: u32,
    pub blank: u32,
    pub comments: u32,
    pub total: u32,
}

pub enum language {
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
use self::language::*;

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


fn get_language(filepath: &str) -> language {
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

enum CommentStyle {
    Simple(Option<&str>, Option<&str,&str>),
    Extensive(Vec<&str>,Vec<(&str,&str)>),
}

use self::CommentStyle::*;

fn get_language_config(language: &language) {
    let c_style = N(Some("//"), Some(("/*", "*/")));
    let html_style = N(None, Some(("<!--", "-->")));
    let no_comments = N(None, None);
    let sh_style = N(Some("#"), None);

    let comment_style = match *language {
        C | Cpp | Rust | Java | Go | Javascript => c_style,
        Assembly => N(Some("#"), Some(("/*", "*/"))),
        Python => N(Some("#"), Some(("'''", "'''"))),
        Text | Markdown | Unknown => no_comments,
        Makefile => sh_style,
        Html=> html_style,
    };
    
}

pub fn count(filepath :&str) -> count {
    let language = get_language(filepath);
    let cfg = get_language_config(&language);

}
