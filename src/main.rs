#[macro_use]
extern crate lazy_static;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug,PartialEq)]
struct Entry {
    text: String,
    phonemes: Vec<String>,
    variant: i32,
}

impl Entry {
    fn new(line: &str) -> Entry {
        let mut trimmed_line = line;
        // Strip comments if present ('#' through the end of line).
        let comment_start = line.find('#');
        if comment_start.is_some() {
            trimmed_line = &line[0..comment_start.unwrap()];
        }

        // Split the rest on whitespace and use regexen to pull out the important parts.
        lazy_static! {
            // This matches the term and optional (N) suffix, e.g. from "aalborg(2)".
            // Capture group 1 has the term text and capture 3 has the integer, if any.
            static ref TERM_RE: Regex = Regex::new(r"([^ ()#]*)(\(([0-9]+)\))?").unwrap();
            // This matches phonemes like "AA1", "N" and "AH0". If there is an integer
            // part, it's a vowel sound.
            static ref PHONEME_RE: Regex = Regex::new(r"[A-Z]+([0-9]+)?").unwrap();
        }
        let tokens: Vec<&str> = trimmed_line.split_whitespace().collect();

        let term_cap = TERM_RE.captures(&tokens[0]).unwrap();
        let mut result = Entry {
            text: String::from(&term_cap[0]),
            phonemes: Vec::with_capacity(tokens.len()-1),
            variant: 1,
        };
        if term_cap.get(3).is_some() {
            result.variant = term_cap[3].parse().unwrap();
        }

        for ph in &tokens[1..] {
            let ph_cap = PHONEME_RE.captures(&ph).unwrap();
            result.phonemes.push(String::from(&ph_cap[0]));
        }

        return result;
    }

    // TODO: This is a very silly way to avoid dealing with string literals in tests.
    fn new2(txt: &str, phnms: &Vec<&str>) -> Entry {
        Entry {
            text: String::from(txt),
            phonemes: phnms.iter().map(|&x| String::from(x)).collect(),
            variant: 1,
        }
    }
}

#[test]
fn test_cmudict_entry_parser() {
    // Test parse of regular word.
    assert_eq!(Entry::new("ampersand AE1 M P ER0 S AE2 N D"),
               Entry::new2("ampersand", &vec!["AE1", "M", "P", "ER0", "S", "AE2", "N", "D"]));
    // Everything after "#" should be ignored.
    assert_eq!(Entry::new("gdp G IY1 D IY1 P IY1 # abbrev"),
               Entry::new2("gdp", &vec!["G", "IY1", "D", "IY1", "P", "IY1"]));
    // Test parse with single quote in term.
    assert_eq!(Entry::new("'frisco F R IH1 S K OW0"),
               Entry::new2("'frisco", &vec!["F", "R", "IH1", "S", "K", "OW0"]));
    // Test parse with periods in term.
    assert_eq!(Entry::new("a.m. EY2 EH1 M"),
               Entry::new2("a.m.", &vec!["EY2", "EH1", "M"]));
    // Test parse of alternate words. No special casing for now.
    assert_eq!(Entry::new("amounted(2) AH0 M AW1 N IH0 D"),
               Entry { text: String::from("amounted(2)"),
               phonemes: vec!["AH0", "M", "AW1", "N", "IH0", "D"].iter().map(|&x| String::from(x)).collect(),
                   variant: 2});
}

// TODO: Idiomatic error handling.
fn read_cmudict_to_lines() -> Vec<String> {
    let f = File::open("./cmudict.dict").unwrap();
    let br = BufReader::new(f);
    let mut v = vec![];
    for line in br.lines() {
        v.push(String::from(line.unwrap().trim()));
    }
    return v;
}

fn main() {
    println!("Hello, world!");

    let cmudict_lines = read_cmudict_to_lines();
    println!("Read {} lines.", cmudict_lines.len());
}
