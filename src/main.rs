extern crate cl;
extern crate clap;
extern crate deque;
extern crate ignore;
extern crate num_cpus;

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::thread;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool,Ordering};
use std::sync::mpsc::channel;
use std::sync::Arc;

use clap::{App, Arg};
use ignore::{WalkBuilder, WalkState};

use deque::{Stealer, Stolen};

use cl::*;

#[derive(PartialEq,Eq)]
enum Sort {
    Blank,
    Code,
    Comment,
    Files,
    Language,
    Lines,
}

impl FromStr for Sort {
    type Err = Option<String>;
    fn from_str(s: &str) -> Result<Sort,Self::Err> {
        match s {
            "blank" | "Blank" => Ok(Sort::Blank),
            "code" | "Code" => Ok(Sort::Code),
            "comment" | "Comment" => Ok(Sort::Comment),
            "lines" | "Lines" => Ok(Sort::Lines),
            "language" | "Language" => Ok(Sort::Language), 
            "files" | "Files" => Ok(Sort::Files),
            _ => Err(None)
        }
    }
}

enum Work {
    File(String),
    Quit,
}

struct Worker {
    stealer: Stealer<Work>,
}

impl Worker {
    fn run(self) -> Vec<FileCount> {
        let mut file_counts: Vec<FileCount> = vec![];
        loop {
            match self.stealer.steal() {
                Stolen::Empty => continue,
                Stolen::Data(Work::Quit) => break,
                Stolen::Data(Work::File(path)) => {
                    let language = get_language(&path);
                    if language != language::Unknown {
                        let count = count(&path);
                        file_counts.push(FileCount {
                            language: language,
                            path: path,
                            count: count,
                        });
                    };
                }
                _ => continue,
            }
        }
        file_counts
    }
}

#[derive(Clone)]
struct FileCount {
    path: String,
    language: language,
    count: Count,
}

struct LanguageTotal {
    files: u32,
    count: Count,
}

fn main() {
    let matches = App::new("Count Lines")
        .version("0.1")
        .author("Markus Åkesson")
        .about("Count lines of code")
        .arg(
            Arg::with_name("files")
                .required(false)
                .long("files")
                .takes_value(false)
                .help("Show stats for each individal file"),
        )
        .arg(
            Arg::with_name("sort")
                .required(false)
                .long("sort")
                .short("s")
                .takes_value(true)
                .value_name("COLUMN")
                .help("Column to short by"),
        )
        .arg(
            Arg::with_name("target")
                .required(true)
                .multiple(true)
                .help("File or directory to count line in (Multiple targets allowed)"),
        )
        .get_matches();

    let targets = match matches.values_of("target") {
        Some(targets) => targets.collect(),
        None => vec!["."],
    };

    let sort: Sort = match matches.value_of("sort") {
        Some(string) => match Sort::from_str(string) {
            Ok(sort) => sort,
            Err(err) => {
                if let Some(suggestion) = err {
                    println!(
                        "Error: invalid value for --sort: {}, perhaps you mean {}?",
                        string, suggestion
                    );
                } else {
                    println!("Error: invalid value for --sort: {}?", string);
                };
                println!(" Hint: valid values are Code, Comment, Blank, Lines, Lanuage and Files. Default: Code.");
                return;
            }
        },
        None => Sort::Code,
    };

    let by_file: bool = matches.is_present("files");

    if by_file && (sort == Sort::Language || sort == Sort::Files) {
        println!("Error: cannot sort by Language or Files when --files is present");
        return;
    }

    let (use_ignore, ignore_hidden) = match matches.occurrences_of("unrestricted") {
        0 => (true, true),
        1 => (false, true),
        2 => (false, false),
        _ => (false, false),
    };

    let threads = num_cpus::get();
    let mut workers = vec![];
    let (workque, stealer) = deque::new();
    for _ in 0..threads {
        let worker = Worker {
            stealer: stealer.clone(),
        };
        workers.push(thread::spawn(|| worker.run()));
    }
    
    let mut paths_iter = targets.iter();
    let first_path = paths_iter.next().expect("Error: Targets cant be empty");
        

    let mut walker = WalkBuilder::new(first_path);
    walker.ignore(use_ignore)
        .git_ignore(use_ignore)
        .git_exclude(use_ignore)
        .hidden(ignore_hidden)
        .threads(threads - 1);

    for path in paths_iter {
        walker.add(path);
    }

    let quit = Arc::new(AtomicBool::new(false));
    let sender_quit = Arc::clone(&quit);
    let (tx,rx) = channel();

    let parallel_walker = walker.build_parallel();
    parallel_walker.run(|| {
        
        let tx_thread = tx.clone();
        let quit = Arc::clone(&sender_quit);
        
        Box::new(move |result| {

            if quit.load(Ordering::Relaxed) {
                return ignore::WalkState::Quit;
            }

            let path = match result {
                Ok(path) => path,
                Err(_) => return WalkState::Continue,
            };

            let ft = match path.file_type() {
                Some(ft) => ft,
                None => return WalkState::Continue,
            };
            if ft.is_file() {
                tx_thread.send(Work::File(String::from(path.path().to_str().unwrap()))).unwrap();
            }

            WalkState::Continue
        })
    });

    drop(tx);
    
    for value in rx {
        workque.push(value);
    }

    for _ in 0..workers.len() {
        workque.push(Work::Quit);
    }
    
    let mut filecounts: Vec<FileCount> = Vec::new();
    for worker in workers {
        filecounts.extend(worker.join().unwrap().iter().cloned());
    }

    let mut by_language: HashMap<language,Vec<FileCount>> = HashMap::new();
    for fc in filecounts {
        match by_language.entry(fc.language) {
            Entry::Occupied(mut elem) => elem.get_mut().push(fc),
            Entry::Vacant(elem) => {
                elem.insert(vec![fc]);
            }
        };
    }

    let linesep = "-".repeat(80);

    if by_file {

        println!("{}",linesep);
        println!(" {0: <17} {1: >8} {2: >12} {3: >12} {4: >12} {5: >12}",
                 "Language",
                 "Files",
                 "Lines",
                 "Blank",
                 "Comments",
                 "Code");
        println!("{}", linesep);

        for (language, mut filecounts) in by_language {
            let mut total = Count::default();
            for fc in &filecounts {
                total.merge(&fc.count);
            }

            println!("{}",linesep);
            println!(" {0: <17} {1: >8} {2: >12} {3: >12} {4: >12} {5: >12}",
                     language,
                     filecounts.len(),
                     total.total,
                     total.blank,
                     total.comments,
                     total.code);
        
             match sort {
                Sort::Code =>
                    filecounts.sort_by(|fc1, fc2| fc2.count.code.cmp(&fc1.count.code)),
                Sort::Comment =>
                    filecounts.sort_by(|fc1, fc2| fc2.count.comments.cmp(&fc1.count.comments)),
                Sort::Blank =>
                    filecounts.sort_by(|fc1, fc2| fc2.count.blank.cmp(&fc1.count.blank)),
                Sort::Lines =>
                    filecounts.sort_by(|fc1, fc2| fc2.count.total.cmp(&fc1.count.total)),
                Sort::Language | Sort::Files => panic!("Sorting by language or files \
                                                        when using the --sort flag"),
             }
        
            println!("{}",linesep);

            for fc in filecounts {
                println!("|{0: <25} {1: >12} {2: >12} {3: >12} {4: >12}",
                         fc.path,
                         fc.count.total,
                         fc.count.blank,
                         fc.count.comments,
                         fc.count.code);
            }
        }
    } else  {

        let mut language_totals: HashMap<&language,LanguageTotal> = HashMap::new();
        for (language, filecounts) in &by_language {

            let mut language_total = Count::default();
            for fc in filecounts {
                language_total.merge(&fc.count);
            }

            language_totals.insert(language,LanguageTotal {
                files: filecounts.len() as u32,
                count: language_total,
            });
        }

        let mut totals_by_language = language_totals.iter().collect::<Vec<(&&language,&LanguageTotal)>>();
        
        match sort {
            Sort::Language => totals_by_language
                .sort_by(|&(l1, _), &(l2, _)| l1.to_string().cmp(&l2.to_string())),
             Sort::Files => totals_by_language
                  .sort_by(|&(_, c1), &(_, c2)| c2.files.cmp(&c1.files)),
            Sort::Code => totals_by_language
                .sort_by(|&(_, c1), &(_, c2)| c2.count.code.cmp(&c1.count.code)),
            Sort::Comment => totals_by_language
                .sort_by(|&(_, c1), &(_, c2)| c2.count.comments.cmp(&c1.count.comments)),
            Sort::Blank => totals_by_language
                .sort_by(|&(_, c1), &(_, c2)| c2.count.blank.cmp(&c1.count.blank)),
            Sort::Lines => totals_by_language
                .sort_by(|&(_, c1), &(_, c2)| c2.count.total.cmp(&c1.count.total)),
        }    
        print_totals_by_language(&linesep,&totals_by_language);
    }
}

fn print_totals_by_language(linesep: &str, totals_by_language: &[(&&language, &LanguageTotal)]) {
    println!("{}", linesep);
    println!(" {0: <17} {1: >8} {2: >12} {3: >12} {4: >12} {5: >12}",
             "Language",
             "Files",
             "Lines",
             "Blank",
             "Comments",
             "Code");
    println!("{}", linesep);

    for &(language, total) in totals_by_language {
        println!(" {0: <17} {1: >8} {2: >12} {3: >12} {4: >12} {5: >12}",
                 language,
                 total.files,
                 total.count.total,
                 total.count.blank,
                 total.count.comments,
                 total.count.code);
    }

    let mut totals = LanguageTotal {
        files: 0,
        count: Count::default(),
    };
    for &(_, total) in totals_by_language {
        totals.files += total.files;
        totals.count.code += total.count.code;
        totals.count.blank += total.count.blank;
        totals.count.comments += total.count.comments;
        totals.count.total += total.count.total;
    }

    println!("{}", linesep);
    println!(" {0: <17} {1: >8} {2: >12} {3: >12} {4: >12} {5: >12}",
             "Total",
             totals.files,
             totals.count.total,
             totals.count.blank,
             totals.count.comments,
             totals.count.code);
    println!("{}", linesep);
}

