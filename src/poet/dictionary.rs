//! A phonetic dictionary.
//!
//! This provides a wrapper around the `cmusphinx` phonetic dictionary.
//!
//! This dictionary uses most of the ARPABET 2-letter phonemes, which are described here:
//! <https://en.wikipedia.org/wiki/ARPABET>
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
//!
//! * <https://github.com/cmusphinx/cmudict>
//! * <https://cmusphinx.github.io/wiki/tutorialdict/>
//! * <http://www.speech.cs.cmu.edu/tools/lextool.html>
//!
use std::cmp::Ordering;
use std::error::Error;

/// An Entry represents a single word or variant with its associated metadata.
///
/// This corresponds to one line in the cmudict file.
#[derive(Clone, Debug, PartialEq)]
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

    fn similarity_key(&self) -> String {
        let mut result = String::with_capacity(4 * 100); // 100 chars should be plenty.
        for ph in self.phonemes.iter().rev() {
            result.push_str(ph);
            result.push(' ');
        }
        result.push_str(&self.text); // To disambiguate homonyms.
        return result;
    }

    fn similarity_prefix(&self, syllable_count: usize) -> String {
        let mut result = String::with_capacity(4 * 100); // 100 chars should be plenty.
        let mut vowel_count: usize = 0;
        for ph in self.phonemes.iter().rev() {
            let is_vowel = ph.matches(char::is_numeric).count() > 0;
            result.push_str(ph);
            result.push(' ');

            if is_vowel {
                vowel_count += 1;
                if vowel_count >= syllable_count {
                    break;
                }
            }
        }
        return result;
    }

    pub fn rhymes_with(&self, other: &Self) -> bool {
        // This is a hack but it effectively compares the last syllable of the two words.
        return self.similarity_prefix(1) == other.similarity_prefix(1);
    }
}

/// Computes a similarity score between two words. Higher scores are more similar.
///
/// Args:
/// * `a_phonemes` - The phonemes for the first word.
/// * `b_phonemes` - The phonemes for the second word.
///
fn similarity_score(a_phonemes: &Vec<String>, b_phonemes: &Vec<String>) -> i32 {
    let mut score: i32 = 0;

    for (a, b) in a_phonemes.iter().rev().zip(b_phonemes.iter().rev()) {
        if a == b {
            score += 1;
        } else {
            break;
        }
    }
    return score;
}

/// A container for a collection of entries.
///
/// Either construct one and populate it with individual entries, or initialize one from
/// a text file in `cmudict.dict` format.
#[derive(Debug)]
pub struct Dictionary {
    entries: std::collections::HashMap<String, Entry>,

    // This stores Entry::similarity_key()s to terms. MUST REMAIN SORTED.
    //
    // e.g. ("L AH0 V AH1 SH shovel", "shovel")
    //
    // TODO: Replace the vector with a BTreeMap or a tree / trie.
    // TODO: Replace the value type with a ref to the Entry.
    reverse_list: Vec<(String, String)>,
}

/// Represents a single word along with associated meta-data.
#[derive(Debug, Eq)]
pub struct SimilarWord {
    /// The word.
    pub word: String,

    /// Larger scores represent higher similarity.
    pub score: i32,
}

/// Return value for Dictionary::similar(), holding all the results.
#[derive(Debug)]
pub struct SimilarResult {
    /// All of the similar words sorted decreasing by similarity.
    pub words: Vec<SimilarWord>,
}

impl Ord for SimilarWord {
    fn cmp(&self, other: &Self) -> Ordering {
        // Want to sort descending by score.
        match self.score.cmp(&other.score) {
            Ordering::Less => Ordering::Greater,
            Ordering::Greater => Ordering::Less,
            // Then ascending by the word text.
            Ordering::Equal => self.word.cmp(&other.word),
        }
    }
}

impl PartialOrd for SimilarWord {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for SimilarWord {
    fn eq(&self, other: &Self) -> bool {
        self.word == other.word && self.score == other.score
    }
}

// TODO: Replace this wasteful and crude similarity algorithm.
//
// The current algorithm works by:
//   - Keep a sorted vector of the reverse phonemes (so similar endings appear adjacent).
//   - Return any terms that share the very last syllable sound.
//
// TODO: Prioritize terms that are more similar (share more phonemes at the tail).
impl Dictionary {
    /// Creates a new empty Dictionary.
    pub fn new() -> Dictionary {
        Dictionary {
            entries: std::collections::HashMap::new(),
            reverse_list: vec![],
        }
    }

    /// Creates a new dictionary, populated from the given text file.
    pub fn new_from_cmudict_file(path: &str) -> Result<Dictionary, Box<dyn Error>> {
        let mut dict = Dictionary::new();

        use std::io::{BufRead, BufReader};
        let f = std::fs::File::open(path)?;
        let br = BufReader::new(f);
        for line in br.lines() {
            dict.insert_internal(Entry::new(line?.trim()));
        }
        dict.reverse_list.sort();
        return Ok(dict);
    }

    /// Inserts a single entry.
    #[cfg(test)]  // TODO: Remove?
    pub fn insert(&mut self, entry: Entry) {
        self.insert_internal(entry);
        self.reverse_list.sort();
    }

    /// Inserts a single entry as though it would appear as a single line of the cmudict file.
    #[cfg(test)]  // TODO: Remove?
    pub fn insert_raw(&mut self, line: &str) {
        let entry = Entry::new(line);
        self.insert(entry);
    }

    /// Inserts all of the items in `lines` as though they were individually insert_raw()d.
    #[cfg(test)]  // TODO: Remove?
    pub fn insert_all(&mut self, lines: &Vec<&str>) {
        for line in lines {
            let entry = Entry::new(line);
            self.insert_internal(entry);
        }
        self.reverse_list.sort();
    }

    fn insert_internal(&mut self, entry: Entry) {
        self.reverse_list
            .push((entry.similarity_key(), entry.text.clone()));
        self.entries.insert(entry.text.clone(), entry);
    }

    /// Returns the entry for the given term, or None.
    pub fn lookup(&self, term: &str) -> Option<&Entry> {
        return self.entries.get(term);
    }

    /// Returns the number of entries in the dictionary.
    #[cfg(test)]  // TODO: Remove?
    pub fn len(&self) -> usize {
        return self.entries.len();
    }

    /// Returns terms that share the last syllable with the given term.
    ///
    /// TODO: Replace the return value with something that doesn't have so many copies.
    /// TODO: Make the similarity more discerning, rather than boolean on the last syllable.
    pub fn similar(&self, term: &str) -> SimilarResult {
        let mut result = SimilarResult { words: vec![] };

        let entry = self.lookup(term);
        if entry.is_none() {
            return result;
        }

        let key_prefix: String = entry.unwrap().similarity_prefix(1 /* syllable */);
        for (prefix, t) in &self.reverse_list {
            if t == term {
                continue;
            }
            if prefix.starts_with(key_prefix.as_str()) {
                let other_entry = self.lookup(t).unwrap();
                result.words.push(SimilarWord {
                    word: t.clone(),
                    score: similarity_score(&entry.unwrap().phonemes, &other_entry.phonemes),
                });
            }
        }
        result.words.sort();
        return result;
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
        let entry = dict.lookup("aardvark").unwrap(); // Or fail.
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
    fn test_similarity_score() {
        let bayous = Entry::new("bayous B AY1 UW0 Z");
        let fondues = Entry::new("fondues F AA1 N D UW0 Z");
        let virtues = Entry::new("virtues V ER1 CH UW0 Z");
        assert_eq!(similarity_score(&bayous.phonemes, &fondues.phonemes), 2);
        assert_eq!(similarity_score(&fondues.phonemes, &virtues.phonemes), 2);

        let diagram = Entry::new("diagram D AY1 AH0 G R AE2 M");
        let polygram = Entry::new("polygram P AA1 L IY2 G R AE2 M");
        let program = Entry::new("program P R OW1 G R AE2 M");
        let programme = Entry::new("programme P R OW1 G R AE2 M");
        assert_eq!(similarity_score(&diagram.phonemes, &polygram.phonemes), 4);
        assert_eq!(similarity_score(&diagram.phonemes, &program.phonemes), 4);
        assert_eq!(similarity_score(&program.phonemes, &programme.phonemes), 7);

        let apple = Entry::new("apple AE1 P AH0 L");
        let apple_s = Entry::new("apple's AE1 P AH0 L Z");
        let apples = Entry::new("apples AE1 P AH0 L Z");
        assert_eq!(similarity_score(&apple.phonemes, &apple_s.phonemes), 0);
        assert_eq!(similarity_score(&apple.phonemes, &apples.phonemes), 0);
        assert_eq!(similarity_score(&apple_s.phonemes, &apples.phonemes), 5);

        let mango = Entry::new("mango M AE1 NG G OW0");
        let mangoes = Entry::new("mangoes M AE1 NG G OW0 Z");
        let mangold = Entry::new("mangold M AE1 N G OW2 L D");
        assert_eq!(similarity_score(&mango.phonemes, &mangoes.phonemes), 0);
        assert_eq!(similarity_score(&mango.phonemes, &mangold.phonemes), 0);
        assert_eq!(similarity_score(&mangoes.phonemes, &mangold.phonemes), 0);
    }

    // This helper calls `dict.similar(query)` and checks that the returned words are `expected`.
    fn assert_similar_terms_are(dict: &Dictionary, query: &str, expected: &Vec<&str>) {
        let result = dict.similar(query);
        let result_words: Vec<String> = result
            .words
            .iter()
            .map(|similar_entry| similar_entry.word.clone())
            .collect();
        assert_eq!(&result_words, expected);
    }

    #[test]
    fn test_similar() {
        let values = vec![
            // These words rhyme.
            "bayous B AY1 UW0 Z",
            "fondues F AA1 N D UW0 Z",
            "virtues V ER1 CH UW0 Z",
            // These do too, but not with the first ones.
            "diagram D AY1 AH0 G R AE2 M",
            "polygram P AA1 L IY2 G R AE2 M",
            "program P R OW1 G R AE2 M",
            "programme P R OW1 G R AE2 M",
            "telegram T EH1 L AH0 G R AE2 M",
            // These are other unrelated words to pad out the dictionary.
            "apple AE1 P AH0 L",
            "apple's AE1 P AH0 L Z",
            "apples AE1 P AH0 L Z",
            "applesauce AE1 P AH0 L S AO2 S",
            "avocado AE2 V AH0 K AA1 D OW0",
            "avocados AE2 V AH0 K AA1 D OW0 Z",
            "cranberry K R AE1 N B EH2 R IY0",
            "guava G W AA1 V AH0",
            "guavas G W AA1 V AH0 Z",
            "mango M AE1 NG G OW0",
            "mangoes M AE1 NG G OW0 Z",
            "mangold M AE1 N G OW2 L D",
        ];
        let mut dict = Dictionary::new();
        dict.insert_all(&values);

        assert_similar_terms_are(&dict, "bayous", &vec!["fondues", "virtues"]);
        assert_similar_terms_are(
            &dict,
            "program",
            &vec!["programme", "diagram", "polygram", "telegram"],
        );
        assert!(dict.similar("guava").words.is_empty());
        assert_similar_terms_are(&dict, "apples", &vec!["apple's"]);
    }

    #[test]
    fn test_similar_with_homonyms() {
        let values = vec![
            "read R EH1 D",
            "reade R EH1 D",
            "red R EH1 D",
            "redd R EH1 D",
        ];
        let mut dict = Dictionary::new();
        dict.insert_all(&values);
        assert_similar_terms_are(&dict, "red", &vec!["read", "reade", "redd"]);
    }

    #[test]
    #[ignore] // It's slow.
    fn test_can_read_entire_cmudict() {
        let _dict = Dictionary::new_from_cmudict_file("./cmudict.dict").unwrap();
        // The test is successful if it doesn't crash.
    }
} // mod tests
