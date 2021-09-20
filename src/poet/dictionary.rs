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
//! In this module, the main trait is Dictionary, which provides lookups for individual words, and
//! also the ability to search for words that rhyme with a given word, by comparing the suffixes of
//! the pronciations. Items in the dictionaries are represented by `Entry`s, each corresponding
//! to one row in the phonetic dictionary.
//!
//! Related references:
//!
//! * <https://github.com/cmusphinx/cmudict>
//! * <https://cmusphinx.github.io/wiki/tutorialdict/>
//! * <http://www.speech.cs.cmu.edu/tools/lextool.html>
//!
use rocket::serde::Serialize;
use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

/// Represents the phonemes of a word, in ARPABET / cmudict format.
#[derive(Clone, Debug, PartialEq)]
pub struct Phonemes {
    /// The individual phonemes as listed, in the original order e.g. `["SH", "R", "IH1", "M", "P"]`.
    pub phonemes: Vec<String>,

    /// The number of syllables, identified by the number of vowel sounds.
    syllables: i32,
}

impl Phonemes {
    fn new() -> Phonemes {
        Phonemes {
            phonemes: vec![],
            syllables: 0,
        }
    }

    /// Initializes from the given sequence.
    ///
    /// Args:
    /// * `tokens` - A slice like &["SH", "R", "IH1", "M", "P"], e.g. from a raw input.
    fn from_slice(&mut self, tokens: &[&str]) {
        self.phonemes = Vec::with_capacity(tokens.len());
        self.syllables = 0;

        use regex::Regex;
        lazy_static! {
            // This matches phonemes like "AA1", "N" and "AH0". If there is an integer
            // part, it's a vowel sound.
            static ref PHONEME_RE: Regex = Regex::new(r"[A-Z]+([0-9]+)?").unwrap();
        }

        for ph in tokens {
            let ph_cap = PHONEME_RE.captures(&ph).unwrap();
            self.phonemes.push(String::from(&ph_cap[0]));
            // Phonemes with integers indicate the main vowel sounds.
            if ph_cap.get(1).is_some() {
                self.syllables += 1;
            }
        }
    }

    /// Returns the number of syllables in the word.
    pub fn num_syllables(&self) -> i32 {
        self.syllables
    }

    /// Returns a string that, when used as a sort key, places similar sequences together.
    ///
    /// This is just the reversed phoneme sequence.
    fn similarity_key(&self) -> String {
        let mut result = String::with_capacity(4 * 100); // 100 chars should be plenty.
        for ph in self.phonemes.iter().rev() {
            result.push_str(ph);
            result.push(' ');
        }
        return result;
    }

    /// Returns the phonemes for the last n syllables, reversed (for sorting).
    ///
    /// This is used for selecting ranges of similar words sorted by similarity_keys.
    fn last_n_syllables(&self, syllable_count: usize) -> String {
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

    /// Returns whether this rhymes with another set of phonemes.
    pub fn rhymes_with(&self, other: &Self) -> bool {
        // This is a hack but it effectively compares the last syllable of the two words.
        return self.last_n_syllables(1) == other.last_n_syllables(1);
    }

    /// Computes a similarity score between two words. Higher scores are more similar.
    ///
    /// Args:
    /// * `a_phonemes` - The phonemes for the first word.
    /// * `b_phonemes` - The phonemes for the second word.
    ///
    fn similarity_score(&self, other: &Self) -> i32 {
        let mut score: i32 = 0;

        for (a, b) in self.phonemes.iter().rev().zip(other.phonemes.iter().rev()) {
            if a == b {
                score += 1;
            } else {
                break;
            }
        }
        return score;
    }
}

impl fmt::Display for Phonemes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.phonemes.join(" "))
    }
}

/// An Entry represents a single word or variant with its associated metadata.
///
/// This corresponds to one line in the cmudict file.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    /// The term as listed in the dictionary, e.g. "flower", "aluminium(2)", "let's", "a.m.".
    pub word: String,
    /// The phonemes of the word e.g. `["SH", "R", "IH1", "M", "P"]`.
    pub phonemes: Phonemes,
    /// The variant, e.g. 2 for the term `aluminium(2)`. Default 1.
    pub variant: i32,
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
        }
        let tokens: Vec<&str> = trimmed_line.split_whitespace().collect();

        let term_cap = TERM_RE.captures(&tokens[0]).unwrap();
        let mut result = Entry {
            word: String::from(&term_cap[1]),
            phonemes: Phonemes::new(),
            variant: 1,
        };
        if term_cap.get(3).is_some() {
            result.variant = term_cap[3].parse().unwrap();
        }

        result.phonemes.from_slice(&tokens[1..]);

        return result;
    }

    /// Returns a string that, when used as a sort key, places similar words together.
    fn similarity_key(&self) -> String {
        // NOTE: This is mostly used as an internal data structure hack that could
        // be done with a comparator.
        let mut key = self.phonemes.similarity_key();
        key.push_str(&self.dict_key()); // To disambiguate homonyms -- dict_key() is unique.
        return key;
    }

    fn similarity_prefix(&self, syllable_count: usize) -> String {
        return self.phonemes.last_n_syllables(syllable_count);
    }

    pub fn rhymes_with(&self, other: &Self) -> bool {
        return self.phonemes.rhymes_with(&other.phonemes);
    }

    pub fn num_syllables(&self) -> i32 {
        return self.phonemes.num_syllables();
    }

    /// Returns the text if the term were in the original dictionary.
    ///
    /// For the first variant, this is just the word. For later variants, it has a "(N)" suffix,
    /// e.g. "foo" and "a(2)".
    pub fn dict_key(&self) -> String {
        if self.variant == 1 {
            return self.word.clone();
        } else {
            return format!("{}({})", self.word, self.variant);
        }
    }
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "\"{}\" ({}); variant={}, syllables={}",
            &self.word,
            &self.phonemes,
            &self.variant,
            self.num_syllables()
        )
    }
}

/// A container for a collection of entries.
///
/// Either construct one and populate it with individual entries, or initialize one from
/// a text file in `cmudict.dict` format.
#[derive(Debug)]
pub struct DictionaryImpl {
    entries: std::collections::HashMap<String, Vec<Entry>>,

    // This stores Entry::similarity_key()s to (term + variant). MUST REMAIN SORTED.
    //
    // e.g. ("L AH0 V AH1 SH shovel", ("shovel", 1))
    //
    // TODO: Replace the vector with a BTreeMap or a tree / trie.
    //
    // NOTE: I attempted to switch the value type to an &Entry, which turned into a
    // lifetime mess. On a deadline; skipping for now.
    reverse_list: Vec<(String, (String, i32))>,
}

/// Represents a single word along with associated meta-data.
// NOTE! If this structure is changed, verify that the templates still render.
#[derive(Clone, Debug, Eq, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct SimilarWord {
    /// The word.
    pub word: String,

    /// The number of syllables in `word`.
    pub syllables: i32,

    /// Larger scores represent higher similarity.
    pub score: i32,
}

/// Return value for Dictionary::similar(), holding all the results.
// NOTE! If this structure is changed, verify that the templates still render.
#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
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

/// The lookup/read/query interface to Dictionaries.
pub trait Dictionary {
    /// Looks up the given term, returning all entries that match.
    ///
    /// Args:
    /// * `term` - a single word of user text, like "chicken" or "let's".
    ///
    /// Returns all the entries that match the term, or None. If there are several entries in the
    /// result, they likely differ in phonemes. The query `era` might return both of these:
    /// ```noformat
    /// era, variant:1, "EH1 R AH0"
    /// era, variant:2, "IH1 R AH0"
    /// ```
    fn lookup(&self, term: &str) -> Option<&Vec<Entry>>;

    /// Looks up a specific `term` and `variant` combination.
    ///
    /// `lookup_variant("era", 2)` would return the "IH1 R AH0" entry above.
    fn lookup_variant(&self, term: &str, variant: i32) -> Option<&Entry>;

    /// Returns a collection of words that are similar to (rhyme with) the given word.
    ///
    /// Args:
    /// * `query` - a single word of user text try and rhyme with.
    ///
    /// The results are ordered in decreasing order of similarity to the query word.
    ///
    /// TODO: Rename "similar" with "rhyme" everywhere, because it's more accurate.
    fn similar(&self, query: &str) -> SimilarResult;
}

// TODO: Replace this wasteful and crude similarity algorithm.
//
// The current algorithm works by:
//   - Keep a sorted vector of the reverse phonemes (so similar endings appear adjacent).
//   - Return any terms that share the very last syllable sound.
//
// TODO: Prioritize terms that are more similar (share more phonemes at the tail).
impl DictionaryImpl {
    /// Creates a new empty Dictionary.
    pub fn new() -> DictionaryImpl {
        DictionaryImpl {
            entries: std::collections::HashMap::new(),
            reverse_list: vec![],
        }
    }

    /// Creates a new dictionary, populated from the given text file.
    pub fn new_from_cmudict_file(path: &str) -> Result<DictionaryImpl, Box<dyn Error>> {
        let mut dict = DictionaryImpl::new();

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
    #[cfg(test)] // TODO: Remove?
    pub fn insert(&mut self, entry: Entry) {
        self.insert_internal(entry);
        self.reverse_list.sort();
    }

    /// Inserts a single entry as though it would appear as a single line of the cmudict file.
    #[cfg(test)] // TODO: Remove?
    pub fn insert_raw(&mut self, line: &str) {
        let entry = Entry::new(line);
        self.insert(entry);
    }

    /// Inserts all of the items in `lines` as though they were individually insert_raw()d.
    #[cfg(test)] // TODO: Remove?
    pub fn insert_all(&mut self, lines: &Vec<&str>) {
        for line in lines {
            let entry = Entry::new(line);
            self.insert_internal(entry);
        }
        self.reverse_list.sort();
    }

    fn insert_internal(&mut self, entry: Entry) {
        self.reverse_list
            .push((entry.similarity_key(), (entry.word.clone(), entry.variant)));
        // word is used in the forward list in order to match as many options as possible from a
        // user's text.
        let key = entry.word.clone();
        if let Some(v) = self.entries.get_mut(&key) {
            v.push(entry);
        } else {
            self.entries.insert(key, vec![entry]);
        }
    }

    /// Returns the number of entries in the dictionary.
    #[cfg(test)] // TODO: Remove?
    pub fn len(&self) -> usize {
        return self.entries.len();
    }
}

impl Dictionary for DictionaryImpl {
    /// Returns the all of the Entries for the given term, or None.
    fn lookup(&self, term: &str) -> Option<&Vec<Entry>> {
        return self.entries.get(term);
    }

    /// Looks up the given term and exact variant.
    fn lookup_variant(&self, term: &str, variant: i32) -> Option<&Entry> {
        if let Some(v) = self.entries.get(term) {
            for entry in v {
                if entry.variant == variant {
                    return Some(&entry);
                }
            }
        }
        return None;
    }

    /// Returns terms that share the last syllable with the given term.
    ///
    /// TODO: Replace the return value with something that doesn't have so many copies.
    /// TODO: Make the filtering more discerning, rather than boolean on the last syllable.
    fn similar(&self, query: &str) -> SimilarResult {
        let mut result = SimilarResult { words: vec![] };

        let query_variants = self.lookup(query);
        if query_variants.is_none() {
            return result;
        }

        // The input query is just the word, so there can be several different pronunciations
        // for the word, each of which has potentially different rhyming words.
        //
        // For example, "our" can be pronounced either to rhyme with "sour" or "far".
        for query_variant in query_variants.unwrap() {
            // Select entries in the reverse_list that have the same last syllable.
            // NOTE: This is a "crude approximation" since it excludes some legitimate rhumes.
            // NOTE: This is a linear scan, which sucks, but it's good enough for now.
            let key_prefix: String = query_variant.similarity_prefix(1 /* syllable */);
            for (prefix, (word, variant)) in &self.reverse_list {
                if !prefix.starts_with(key_prefix.as_str()) {
                    continue;
                }
                if word == query {
                    continue; // Ignore self-syns.
                }

                // This is an exact lookup for the other word variant.
                // Not-None because reverse_list should be 1:1 with the main map.
                let potential_rhyme = self.lookup_variant(&word, *variant).unwrap();
                let score = query_variant
                    .phonemes
                    .similarity_score(&potential_rhyme.phonemes);
                result.words.push(SimilarWord {
                    word: word.clone(),
                    syllables: potential_rhyme.num_syllables(),
                    score: score,
                });
            }
        }

        // TODO: This mashes everything together, which is not great. Switch to grouped results.
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
        assert_eq!(entry.word, "ampersand");
        assert_eq!(entry.dict_key(), "ampersand");
        assert_eq!(
            entry.phonemes.phonemes,
            vec!["AE1", "M", "P", "ER0", "S", "AE2", "N", "D"]
        );
    }

    #[test]
    fn test_parser_ignores_comments() {
        // Everything after # should be ignored.
        let entry = Entry::new("gdp G IY1 D IY1 P IY1 # abbrev ## IGN");
        assert_eq!(entry.word, "gdp");
        assert_eq!(
            entry.phonemes.phonemes,
            vec!["G", "IY1", "D", "IY1", "P", "IY1"]
        );
    }

    #[test]
    fn test_parser_with_punctuation_in_terms() {
        assert_eq!(Entry::new("'frisco F R IH1 S K OW0").word, "'frisco");
        assert_eq!(Entry::new("a.m. EY2 EH1 M").word, "a.m.");
    }

    #[test]
    fn test_parser_with_alternate_words() {
        let entry = Entry::new("amounted(2) AH0 M AW1 N IH0 D");
        assert_eq!(entry.word, "amounted");
        assert_eq!(entry.dict_key(), "amounted(2)");
        assert_eq!(
            entry.phonemes.phonemes,
            vec!["AH0", "M", "AW1", "N", "IH0", "D"]
        );
        assert_eq!(entry.variant, 2);
    }

    #[test]
    fn test_default_variant_is_one() {
        let entry = Entry::new("a AH0");
        assert_eq!(entry.variant, 1);
    }

    #[test]
    fn test_entry_syllable_count() {
        assert_eq!(Entry::new("a AH0").num_syllables(), 1);
        assert_eq!(Entry::new("aardvark AA1 R D V AA2 R K").num_syllables(), 2);
        assert_eq!(
            Entry::new("amounted(2) AH0 M AW1 N IH0 D").num_syllables(),
            3
        );
        assert_eq!(Entry::new("gdp G IY1 D IY1 P IY1").num_syllables(), 3);
    }

    #[test]
    fn test_dictionary_insert() {
        let mut dict = DictionaryImpl::new();
        dict.insert(Entry::new("a AH0"));
        dict.insert(Entry::new("aardvark AA1 R D V AA2 R K"));
        dict.insert(Entry::new("aardvarks AA1 R D V AA2 R K S"));
        assert_eq!(dict.len(), 3);
    }

    #[test]
    fn test_dictionary_insert_raw() {
        let mut dict = DictionaryImpl::new();
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
        let mut dict = DictionaryImpl::new();
        dict.insert_all(&values);
        assert_eq!(dict.len(), 3);
    }

    #[test]
    fn test_dictionary_lookup_by_term() {
        let mut dict = DictionaryImpl::new();
        dict.insert(Entry::new("a AH0"));
        dict.insert(Entry::new("aardvark AA1 R D V AA2 R K"));
        dict.insert(Entry::new("aardvarks AA1 R D V AA2 R K S"));
        let entry = dict.lookup("aardvark").unwrap(); // Or fail.
        assert_eq!(entry[0].word, "aardvark");
        assert_eq!(entry[0].phonemes.phonemes.len(), 7);
        assert_eq!(None, dict.lookup("unknown"));
    }

    #[test]
    fn test_dictionary_lookup_multi() {
        let values = vec!["our AW1 ER0", "our(2) AW1 R", "our(3) AA1 R"];
        let mut dict = DictionaryImpl::new();
        dict.insert_all(&values);
        let entries = dict.lookup("our").unwrap(); // Or fail.
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].word, "our");
        assert_eq!(entries[1].word, "our");
        assert_eq!(entries[2].word, "our");
        assert_eq!(entries[0].dict_key(), "our");
        assert_eq!(entries[1].dict_key(), "our(2)");
        assert_eq!(entries[2].dict_key(), "our(3)");
        assert_eq!(entries[0].variant, 1);
        assert_eq!(entries[1].variant, 2);
        assert_eq!(entries[2].variant, 3);
        assert_eq!(entries[1].phonemes.phonemes, vec!["AW1", "R"]);
    }

    #[test]
    fn test_dictionary_lookup_variant() {
        let values = vec!["our AW1 ER0", "our(2) AW1 R", "our(3) AA1 R"];
        let mut dict = DictionaryImpl::new();
        dict.insert_all(&values);
        assert_eq!(dict.lookup_variant("our", 2).unwrap().dict_key(), "our(2)");
        assert_eq!(dict.lookup_variant("our", 1).unwrap().dict_key(), "our");
        assert_eq!(dict.lookup_variant("our", 9), None);
        assert_eq!(dict.lookup_variant("foo", 1), None);
    }

    #[test]
    fn test_dictionary_stats() {
        let mut dict = DictionaryImpl::new();
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
        assert_eq!(bayous.phonemes.similarity_score(&fondues.phonemes), 2);
        assert_eq!(fondues.phonemes.similarity_score(&virtues.phonemes), 2);

        let diagram = Entry::new("diagram D AY1 AH0 G R AE2 M");
        let polygram = Entry::new("polygram P AA1 L IY2 G R AE2 M");
        let program = Entry::new("program P R OW1 G R AE2 M");
        let programme = Entry::new("programme P R OW1 G R AE2 M");
        assert_eq!(diagram.phonemes.similarity_score(&polygram.phonemes), 4);
        assert_eq!(diagram.phonemes.similarity_score(&program.phonemes), 4);
        assert_eq!(program.phonemes.similarity_score(&programme.phonemes), 7);

        let apple = Entry::new("apple AE1 P AH0 L");
        let apple_s = Entry::new("apple's AE1 P AH0 L Z");
        let apples = Entry::new("apples AE1 P AH0 L Z");
        assert_eq!(apple.phonemes.similarity_score(&apple_s.phonemes), 0);
        assert_eq!(apple.phonemes.similarity_score(&apples.phonemes), 0);
        assert_eq!(apple_s.phonemes.similarity_score(&apples.phonemes), 5);

        let mango = Entry::new("mango M AE1 NG G OW0");
        let mangoes = Entry::new("mangoes M AE1 NG G OW0 Z");
        let mangold = Entry::new("mangold M AE1 N G OW2 L D");
        assert_eq!(mango.phonemes.similarity_score(&mangoes.phonemes), 0);
        assert_eq!(mango.phonemes.similarity_score(&mangold.phonemes), 0);
        assert_eq!(mangoes.phonemes.similarity_score(&mangold.phonemes), 0);
    }

    // This helper calls `dict.similar(query)` and checks that the returned words are `expected`.
    fn assert_similar_terms_are(dict: &DictionaryImpl, query: &str, expected: &Vec<&str>) {
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
        let mut dict = DictionaryImpl::new();
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
        let mut dict = DictionaryImpl::new();
        dict.insert_all(&values);
        assert_similar_terms_are(&dict, "red", &vec!["read", "reade", "redd"]);
    }

    #[test]
    fn test_similar_returns_words_for_all_variants() {
        // These are fake pronunciations to trigger the case where
        let values = vec![
            "far F AA1 R",
            "far's F AA1 R Z",
            "our AW1 ER0",
            "our(2) AW1 R",
            "our(3) AA1 R",
            "sour S AW1 ER0",
            "sour(2) S AW1 R",
        ];
        let mut dict = DictionaryImpl::new();
        dict.insert_all(&values);

        // Case: A word with a single variant should rhyme with variants of other words.
        assert_similar_terms_are(&dict, "far", &vec!["our" /* (3) */]);

        // Case: A word with many variants should return words that are similar to any variant.
        // "sour" appears twice because both variants rhyme. (At the moment they aren't
        // distinguishable because the phonemes aren't passed back.)
        assert_similar_terms_are(&dict, "our", &vec!["far", "sour", "sour"]);
    }

    #[test]
    #[ignore] // It's slow.
    fn test_can_read_entire_cmudict() {
        let _dict = DictionaryImpl::new_from_cmudict_file("./cmudict.dict").unwrap();
        // The test is successful if it doesn't crash.
    }
} // mod tests
