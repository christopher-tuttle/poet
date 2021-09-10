#[macro_use]
extern crate lazy_static;
extern crate clap;
use clap::{App, Arg};
use std::fs::File;
use std::io::{BufRead, BufReader};

mod poet;

use crate::poet::*;


fn handle_term_query(query: &str, dict: &dictionary::Dictionary) {
    if let Some(entry) = dict.lookup(query) {
        println!("Found {:?}", entry);
    } else {
        println!("Not found: {}", query);
    }
}

fn handle_input_file(path: &str, dict: &dictionary::Dictionary) {
    let f = File::open(path).unwrap();
    let br = BufReader::new(f);
    for line in br.lines() {
        if line.as_ref().unwrap().is_empty() {
            continue;
        }
        println!("{}", line.as_ref().unwrap());
        // TODO: Replace this with a real tokenizer. It misses out due
        // to punctuation (including [.,-!/]), and capitalization.
        //
        // There's also a case with hyphenates at the ends of lines,
        // but this is probably a later problem.
        let mut num_syllables: i32 = 0;
        for token in line.as_ref().unwrap().trim().split_whitespace() {
            let key = snippet::normalize_for_lookup(token);
            if let Some(entry) = dict.lookup(&key) {
                println!("\t{}: {:?}", token, entry);
                num_syllables += entry.syllables;
            } else {
                println!("\t{}: None", token);
            }
        }
        println!("\t==> Line summary: {} syllables.", num_syllables);
        println!("");
    }
}

fn main() {
    let matches = App::new("poet")
        .version("0.1.0")
        .arg(
            Arg::with_name("dict")
                .short("d")
                .long("dict")
                .value_name("FILE")
                .help("Path to the cmudict.dict dictionary file.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("query")
                .short("q")
                .long("query")
                .value_name("WORD")
                .help("Looks up WORD in the dictionary and returns info on it.")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("Analyzes the lines in the given text file.")
                .takes_value(true)
        )
        .get_matches();

    let cmudict_path = matches.value_of("dict").unwrap_or("./cmudict.dict");

    println!("Hello, world!");

    let dict = poet::dictionary::Dictionary::new_from_cmudict_file(cmudict_path)
        .expect("Failed to read cmudict file!");

    if let Some(q) = matches.value_of("query") {
        // TODO: Exit with a failure status value if lookup failed.
        handle_term_query(q, &dict);
    }

    if let Some(path) = matches.value_of("input") {
        // TODO: Handle errors more gracefully.
        handle_input_file(path, &dict);
    }
}
