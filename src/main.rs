#[macro_use]
extern crate lazy_static;
extern crate clap;
use clap::{App, Arg};
use std::fs::File;
use std::io::{BufRead, BufReader};

pub mod poet {
pub mod dictionary {

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub text: String,
    pub phonemes: Vec<String>,
    pub variant: i32,
    pub syllables: i32,
}

impl Entry {
    pub fn new(line: &str) -> Entry {
        let mut trimmed_line = line;
        // Strip comments if present ('#' through the end of line).
        let comment_start = line.find('#');
        if comment_start.is_some() {
            trimmed_line = &line[0..comment_start.unwrap()];
        }

        // Split the rest on whitespace and use regexen to pull out the important parts.
        use regex::Regex;
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
            phonemes: Vec::with_capacity(tokens.len() - 1),
            variant: 1,
            syllables: 0,
        };
        if term_cap.get(3).is_some() {
            result.variant = term_cap[3].parse().unwrap();
        }

        for ph in &tokens[1..] {
            let ph_cap = PHONEME_RE.captures(&ph).unwrap();
            result.phonemes.push(String::from(&ph_cap[0]));
            // Phonemes with integers indicate the main vowel sounds.
            if ph_cap.get(1).is_some() {
                result.syllables += 1;
            }
        }

        return result;
    }
}

#[derive(Debug)]
pub struct Dictionary {
    entries: std::collections::HashMap<String, Entry>,
}

impl Dictionary {
    pub fn new() -> Dictionary {
        Dictionary {
            entries: std::collections::HashMap::new()
        }
    }

    pub fn insert(&mut self, entry: Entry) {
        self.entries.insert(entry.text.clone(), entry);
    }

    pub fn insert_raw(&mut self, line: &str) {
        let entry = Entry::new(line);
        self.entries.insert(entry.text.clone(), entry);
    }

    pub fn insert_all(&mut self, lines: &Vec<&str>) {
        for line in lines {
            self.insert_raw(line);
        }
    }

    pub fn lookup(&self, term: &str) -> Option<&Entry> {
        return self.entries.get(term);
    }

    pub fn len(&self) -> usize {
        return self.entries.len();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_handles_basic_entries() {
        let entry = Entry::new("ampersand AE1 M P ER0 S AE2 N D");
        assert_eq!(entry.text, "ampersand");
        assert_eq!(
            entry.phonemes,
            vec!["AE1", "M", "P", "ER0", "S", "AE2", "N", "D"]
        );
    }

    #[test]
    fn test_parser_ignores_comments() {
        // Everything after # should be ignored.
        let entry = Entry::new("gdp G IY1 D IY1 P IY1 # abbrev ## IGN");
        assert_eq!(entry.text, "gdp");
        assert_eq!(entry.phonemes, vec!["G", "IY1", "D", "IY1", "P", "IY1"]);
    }

    #[test]
    fn test_parser_with_punctuation_in_terms() {
        assert_eq!(Entry::new("'frisco F R IH1 S K OW0").text, "'frisco");
        assert_eq!(Entry::new("a.m. EY2 EH1 M").text, "a.m.");
    }

    #[test]
    fn test_parser_with_alternate_words() {
        let entry = Entry::new("amounted(2) AH0 M AW1 N IH0 D");
        assert_eq!(entry.text, "amounted(2)");
        assert_eq!(entry.phonemes, vec!["AH0", "M", "AW1", "N", "IH0", "D"]);
        assert_eq!(entry.variant, 2);
    }

    #[test]
    fn test_default_variant_is_one() {
        let entry = Entry::new("a AH0");
        assert_eq!(entry.variant, 1);
    }

    #[test]
    fn test_entry_syllable_count() {
        assert_eq!(Entry::new("a AH0").syllables, 1);
        assert_eq!(Entry::new("aardvark AA1 R D V AA2 R K").syllables, 2);
        assert_eq!(Entry::new("amounted(2) AH0 M AW1 N IH0 D").syllables, 3);
        assert_eq!(Entry::new("gdp G IY1 D IY1 P IY1").syllables, 3);
    }

    #[test]
    fn test_dictionary_insert() {
        let mut dict = Dictionary::new();
        dict.insert(Entry::new("a AH0"));
        dict.insert(Entry::new("aardvark AA1 R D V AA2 R K"));
        dict.insert(Entry::new("aardvarks AA1 R D V AA2 R K S"));
        assert_eq!(dict.len(), 3);
    }

    #[test]
    fn test_dictionary_insert_raw() {
        let mut dict = Dictionary::new();
        dict.insert_raw("a H0");
        dict.insert_raw("a.m. EY2 EH1 M");
        assert_eq!(dict.len(), 2);
    }

    #[test]
    fn test_dictionary_insert_all() {
        let values: Vec<&str> = vec![
            "a AH0",
            "aardvark AA1 R D V AA2 R K",
            "aardvarks AA1 R D V AA2 R K S",
        ];
        let mut dict = Dictionary::new();
        dict.insert_all(&values);
        assert_eq!(dict.len(), 3);
    }

    #[test]
    fn test_dictionary_lookup_by_term() {
        let mut dict = Dictionary::new();
        dict.insert(Entry::new("a AH0"));
        dict.insert(Entry::new("aardvark AA1 R D V AA2 R K"));
        dict.insert(Entry::new("aardvarks AA1 R D V AA2 R K S"));
        let entry = dict.lookup("aardvark").unwrap();  // Or fail.
        assert_eq!(entry.text, "aardvark");
        assert_eq!(entry.phonemes.len(), 7);
        assert_eq!(None, dict.lookup("unknown"));
    }

    #[test]
    fn test_dictionary_stats() {
        let mut dict = Dictionary::new();
        dict.insert(Entry::new("a AH0"));
        dict.insert(Entry::new("aardvark AA1 R D V AA2 R K"));
        dict.insert(Entry::new("aardvarks AA1 R D V AA2 R K S"));
        assert_eq!(dict.len(), 3);
    }

} // mod tests

}  // mod dictionary
}  // mod poet

use crate::poet::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // It's slow.
    fn test_can_read_entire_cmudict() {
        let lines = read_cmudict_to_lines("./cmudict.dict");
        for line in lines {
            let entry = dictionary::Entry::new(&line);
            println!("Read {:?}", entry);
        }
        // The test is successful if it doesn't crash.
    }

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

// TODO: Idiomatic error handling.
fn read_cmudict_to_lines(path: &str) -> Vec<String> {
    let f = File::open(path).unwrap();
    let br = BufReader::new(f);
    let mut v = vec![];
    for line in br.lines() {
        v.push(String::from(line.unwrap().trim()));
    }
    return v;
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

    let cmudict_lines = read_cmudict_to_lines(cmudict_path);
    println!("Read {} lines.", cmudict_lines.len());

    let mut dict = poet::dictionary::Dictionary::new();
    dict.insert_all(&cmudict_lines.iter().map(|s| s as &str).collect());

    if let Some(q) = matches.value_of("query") {
        // TODO: Exit with a failure status value if lookup failed.
        handle_term_query(q, &dict);
    }

    if let Some(path) = matches.value_of("input") {
        // TODO: Handle errors more gracefully.
        handle_input_file(path, &dict);
    }
}
