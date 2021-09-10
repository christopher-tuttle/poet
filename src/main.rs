#[macro_use]
extern crate lazy_static;
extern crate clap;
use clap::{App, Arg};
use regex::Regex;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, PartialEq)]
struct Entry {
    text: String,
    phonemes: Vec<String>,
    variant: i32,
    syllables: i32,
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
struct Dictionary {
    entries: HashMap<String, Entry>,
}

impl Dictionary {
    fn new() -> Dictionary {
        Dictionary {
            entries: HashMap::new()
        }
    }

    fn insert(&mut self, entry: Entry) {
        self.entries.insert(entry.text.clone(), entry);
    }

    fn insert_raw(&mut self, line: &str) {
        let entry = Entry::new(line);
        self.entries.insert(entry.text.clone(), entry);
    }

    fn insert_all(&mut self, lines: &Vec<&str>) {
        for line in lines {
            self.insert_raw(line);
        }
    }

    fn lookup(&self, term: &str) -> Option<&Entry> {
        return self.entries.get(term);
    }

    fn len(&self) -> usize {
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
    #[ignore] // It's slow.
    fn test_can_read_entire_cmudict() {
        let lines = read_cmudict_to_lines("./cmudict.dict");
        for line in lines {
            let entry = Entry::new(&line);
            println!("Read {:?}", entry);
        }
        // The test is successful if it doesn't crash.
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

fn handle_term_query(query: &str, dict: &Dictionary) {
    if let Some(entry) = dict.lookup(query) {
        println!("Found {:?}", entry);
    } else {
        println!("Not found: {}", query);
    }
}

fn handle_input_file(path: &str, dict: &Dictionary) {
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
            if let Some(entry) = dict.lookup(token) {
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

    let mut dict = Dictionary::new();
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
