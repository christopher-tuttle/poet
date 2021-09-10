#[macro_use]
extern crate lazy_static;
extern crate clap;
use clap::{App, Arg};
use std::fs::File;
use std::io::{BufRead, BufReader};

mod poet;

use crate::poet::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_normalize() {
        assert_eq!(normalize_for_lookup("tools"), "tools");
        assert_eq!(normalize_for_lookup("let's"), "let's");

        // ASCII capital letters should be lowercased.
        assert_eq!(normalize_for_lookup("Such"), "such");
        assert_eq!(normalize_for_lookup("ID"), "id");

        // Trailing punctuation should be cleared in most cases.
        assert_eq!(normalize_for_lookup("this,"), "this");
        assert_eq!(normalize_for_lookup("there."), "there");
        assert_eq!(normalize_for_lookup("found..."), "found");
        assert_eq!(normalize_for_lookup("prize!"), "prize");
        assert_eq!(normalize_for_lookup("flowers?"), "flowers");

        // Periods should be preserved if they also appear within the term.
        assert_eq!(normalize_for_lookup("A.M."), "a.m.");
        assert_eq!(normalize_for_lookup("p.m.,"), "p.m.");
    }
}

/// Normalizes the input word for looking up in the dictionary.
///
/// The words in the dictionary are lower-cased and have only essential punctuation
/// (e.g. "let's" and "a.m."). This cleans up `term` by lower-casing it and
/// stripping punction that's not in the dictionary (e.g. ! and ,), and removing
/// trailing periods if those aren't also found in the word itself.
///
/// # Arguments
///
/// * `term` - A string slice containing a single word.
///
/// # Examples
///
/// ```
/// println!("The normalized form of {} is {}.", "Hello!", normalize_for_lookup("Hello!"));
/// ```
///
fn normalize_for_lookup(term: &str) -> String {
    let mut result = term.to_lowercase();

    // Detect cases like "A.M.". If there is a period in the middle of the term
    // somewhere, then the periods won't be stripped below.
    let mut has_inner_periods = false;
    let mut found_period = false;
    for c in result.chars() {
        if c == '.' {
            found_period = true;
        } else if found_period && c.is_alphanumeric() {
            has_inner_periods = true;
            break;
        }
    }
    
    // NOTE: Keep the period as the first element, as it sliced away below.
    const PUNCTUATION_TO_STRIP: &[char] = &['.', '!', ',', '?'];
    // This makes a copy of the string but remove_matches() is an experimental api still.
    if has_inner_periods {
        result = result.replace(&PUNCTUATION_TO_STRIP[1..], "");
    } else {
        result = result.replace(&PUNCTUATION_TO_STRIP[..], "");
    }

    return result;
}

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
            let key = normalize_for_lookup(token);
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
