//! A phonetic dictionary.
//!
//! This provides a wrapper around the `cmusphinx` phonetic dictionary.
//!
//! This dictionary uses most of the ARPABET 2-letter phonemes, which are described here:
//! https://en.wikipedia.org/wiki/ARPABET
//!
//! In short, entries in the dictionary have a term and pronciation, with the option
//! of having several different pronunciations for a word:
//!
//! ```
//! aluminium AH0 L UW1 M IH0 N AH0 M
//! aluminium(2) AE2 L Y UW1 M IH0 N AH0 M
//! ```
//!
//! The phonemes are classified as either vowel or consonant sounds. Vowel sounds in the
//! dictionary include a stress notation as an integer, e.g. `AH0`. Zero indicates no
//! stress, one is primary stress, two is secondary, and so on. This is used to guess
//! the location and count of the syllables in a word.
//!
//! In this module, the main object is Dictionary, which provides lookups for individual words, and
//! also the ability to search for words that rhyme with a given word, by comparing the suffixes of
//! the pronciations.
//!
//! Related references:
//! https://github.com/cmusphinx/cmudict
//! https://cmusphinx.github.io/wiki/tutorialdict/
//! http://www.speech.cs.cmu.edu/tools/lextool.html
//!
use std::error::Error;

/// An Entry represents a single word or variant with its associated metadata.
///
/// This corresponds to one line in the cmudict file.
#[derive(Debug, PartialEq)]
pub struct Entry {
    /// The term as listed in the dictionary, e.g. "flower", "aluminium(2)", "let's", "a.m.".
    pub text: String,
    /// The individual phonemes as listed, in the original order e.g. `["SH", "R", "IH1", "M", "P"]`.
    pub phonemes: Vec<String>,
    /// The variant, e.g. 2 for the term `aluminium(2)`. Default 1.
    pub variant: i32,
    /// The number of syllables, identified by the number of vowel sounds.
    pub syllables: i32,
}

impl Entry {
    /// Constructs an Entry from the given line, assumed to be in cmudict format.
    ///
    /// Example inputs:
    /// ```
    /// 'twas T W AH1 Z
    /// a AH0
    /// a(2) EY1
    /// a's EY1 Z
    /// a.'s EY1 Z
    /// a.m. EY2 EH1 M
    /// achill AE1 K IH0 L # place, irish
    /// achill's AE1 K IH0 L Z
    /// ```
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

/// A container for a collection of entries.
///
/// Either construct one and populate it with individual entries, or initialize one from
/// a text file in `cmudict.dict` format.
#[derive(Debug)]
pub struct Dictionary {
    entries: std::collections::HashMap<String, Entry>,
}

impl Dictionary {
    /// Creates a new empty Dictionary.
    pub fn new() -> Dictionary {
        Dictionary {
            entries: std::collections::HashMap::new()
        }
    }

    /// Creates a new dictionary, populated from the given text file.
    pub fn new_from_cmudict_file(path: &str) -> Result<Dictionary,Box<dyn Error>> {
        let mut dict = Dictionary::new();

        use std::io::{BufReader, BufRead};
        let f = std::fs::File::open(path)?;
        let br = BufReader::new(f);
        for line in br.lines() {
            dict.insert_raw(line?.trim());
        }
        return Ok(dict);
    }

    /// Inserts a single entry.
    pub fn insert(&mut self, entry: Entry) {
        self.entries.insert(entry.text.clone(), entry);
    }

    /// Inserts a single entry as though it would appear as a single line of the cmudict file.
    pub fn insert_raw(&mut self, line: &str) {
        let entry = Entry::new(line);
        self.entries.insert(entry.text.clone(), entry);
    }

    /// Inserts all of the items in `lines` as though they were individually insert_raw()d.
    pub fn insert_all(&mut self, lines: &Vec<&str>) {
        for line in lines {
            self.insert_raw(line);
        }
    }

    /// Returns the entry for the given term, or None.
    pub fn lookup(&self, term: &str) -> Option<&Entry> {
        return self.entries.get(term);
    }

    /// Returns the number of entries in the dictionary.
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

    #[test]
    #[ignore] // It's slow.
    fn test_can_read_entire_cmudict() {
        let _dict = Dictionary::new_from_cmudict_file("./cmudict.dict").unwrap();
        // The test is successful if it doesn't crash.
    }
} // mod tests

