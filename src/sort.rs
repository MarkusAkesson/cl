enum Sort {
    Blank,
    Code,
    Comment,
    Files,
    Language,
    Lines,
}

impl FromStr for Sort {
    type Err = Option<String>;
    fn from_str(s: &str) -> Result<Sort,Self::Err) {
        match s {
            "blank" | "Blank" => Ok(Sort::Blank),
            "code" | "Code" => Ok(Sort::Code),
            "comment" | "Comment" => Ok(Sort::Comment),
            "lines" | "Lines" => Ok(Sort::Lines),
            "files" | "Files" => Ok(Sort::Files),
            _ => Err(None)
        }
    }
}


