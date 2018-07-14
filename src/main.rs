#[macro_use]
extern crate deque;
extern crate num_cpus;
extern crate clap;

use std::env;
use std::io;
use std::fs::{self, DirEntry, ReadDir,File};
use std::path::Path;
use std::io::{BufRead, BufReader};
use std::thread;

use ignore::{WalkBuilder,WalkState};
use clap::{Arg, App, SubCommand};

use deque::{Stealer,Stolen};

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
                    println!("{}",path);
                    file_counts.push(FileCount {path});
                },
                _ => continue,
            }
        }
        file_counts
    }
}

struct FileCount {
    path: String,
    //language: language,
    //count: count,
}

fn main() {

    let matches = App::new("Count Lines")
                          .version("0.1")
                          .author("Markus Åkesson")
                          .about("Count lines of code")
                          .arg(Arg::with_name("files")
                               .required(false)
                               .long("files")
                               .takes_value(false)
                               .help("Show stats for each individal file"))
                          .arg(Arg:::with_name("sort")
                                .required(false)
                                .long("sort")
                                .short("s")
                                .take_value(true)
                                .value_name("COLUMN")
                                .help("Column to short by"))
                          .arg(Arg::with_name("target")
                                .required(true)
                                .multiple(true)
                                .help("File or directory to count line in (Multiple targets allowed)"))
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
                    println!("Error: invalid value for --sort: '{}', perhaps you mean '{}'?",string suggestion);
                } else {
                    println!("Error: invalid value for --sort: '{}'?",string);
                };
                println!(" Hint: valid values are Code, Comment, Blank, Lines, Lanuage and Files. Default: Code.");
                return
            },
        },
        None => Sort::Code,
    };


    let by_file: bool = matches.is_present("files");

    if by_file && (sort == Sort::Langague || sort == Sort::Files) {
        println!("Error: cannot sort by Language or Files when --files is present");
        return
    }

    let (use_ignore, ignore_hidden) = match matches.occurrences_of("unrestricted") {
        0 => (true, true),
        1 => (false, true),
        2 => (false, false),
        _ => (false, false),
    };
    

    let threads = num_cpus::get();
    let mut workers = vec![];
    let (workque,stealer) = deque::new();
    for _ in 0..threads {
        let worker = Worker {stealer: stealer.clone() };
        workers.push(thread::spawn(||worker.run()));
    }

    let Some((path,paths)) = targets.slit_first();

    let mut walker = WalkBuilder::new(path).ignore(use_ignore)
            .git_ignore(use_ignore)
            .git_exclude(use_ignore)
            .hidden(ignore_hidden)
            .threads(threads)
            .build_parallel(),

   for path in paths {
       walker.add(path);
   }

    parallel_walker.run(|| {
        Box::new(move |result| {
                let path = match result {
                    Ok(path) => path,
                    Err(...) => return WalkState::Continue,
                };

                if path.is_file() {
                    workq.push(Work::File(path));
                }
                    
                WalkState::Continue 
        });
    });



    for _ in 0..workers.len() {
        workque.push(Work::Quit);
    }


}

