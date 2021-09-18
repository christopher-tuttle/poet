//! A "snippet" refers to any bit of text to be analyzed.
//!
//! It is most often one or more poems or other prose, but it can also be a single word.
//! The parser looks for one or more "stanzas" within the snippet, where a stanza is
//! expected to be a contiguous, multi-line text block, often with some poetic form.
//!
//! Generally, the text in a snippet will have punctuation, capitalization,
//! and other formatting that may have to be removed.
use crate::poet::dictionary::*;

/// A token is one word from the original text, normalized and annotated.
#[derive(Debug)]
pub struct Token<'a> {
    /// The text, after passing through `normalize_for_lookup`.
    pub text: String,

    /// The corresponding `dictionary::Entry` if the word is known.
    ///
    /// Lifetime note: holds a reference into the dictionary used for lookup.
    pub entry: Option<&'a Entry>,
    // TODO: Include span information referring to the char positions in raw_text?
    // pub raw_text: &str[],
}

/// Represents a single line of a stanza.
#[derive(Debug)]
pub struct Line<'a> {
    /// The original, user-entered text for the full line.
    pub raw_text: String,

    /// The line number of the orignal line, 1-indexed.
    pub num: usize,

    /// The tokenized version of `raw_text`.
    pub tokens: Vec<Token<'a>>,
}

impl<'a> Line<'a> {
    /// Analyzes the given line and produces a new `Line` with the results.
    ///
    /// Arguments:
    /// * `raw` - A non-empty line from a poem, e.g. `Roses are red,`.
    /// * `line_num` - The 1-indexed line number from the original source.
    /// * `dict` - The dictionary to use for word lookups.
    ///
    /// Lifetime note: the `Dictionary` must outlive the returned `Line`.
    fn new_from_line<'b>(raw: &str, line_num: usize, dict: &'b Dictionary) -> Line<'b> {
        let mut result = Line {
            raw_text: raw.to_string(),
            num: line_num,
            tokens: vec![],
        };

        for word in raw.split_whitespace() {
            let normalized_text = normalize_for_lookup(&word);
            let entry_option = dict.lookup(&normalized_text);
            result.tokens.push(Token {
                text: normalized_text,
                entry: entry_option,
            });
        }
        result
    }

    /// Returns the number of syllables in the line.
    ///
    /// TODO: This may be invalid in face of variants and it ignores unknown words.
    pub fn num_syllables(&self) -> i32 {
        let mut num_syllables = 0;
        for token in &self.tokens {
            if let Some(entry) = &token.entry {
                num_syllables += entry.num_syllables();
            }
        }
        return num_syllables;
    }

    /// Returns whether there are any unknown words in the line.
    pub fn has_unknown_words(&self) -> bool {
        self.tokens.iter().filter(|x| x.entry.is_none()).count() > 0
    }
}

/// Container a block of text (usually a single poem) and its analysis.
///
/// This is the top-level analysis object, holding information about the stanza as a whole. Within
/// it are individual `Line`s, each with `Token`s for each word.
///
/// This implementation assumes Stanzas are fairly short-lived, e.g. that they are created when
/// processing an individual file or web request, and then discarded. Stanzas hold Tokens, which
/// hold references to dictionary `Entry`s.
pub struct Stanza<'a> {
    /// The annotated text of the Stanza.
    pub lines: Vec<Line<'a>>,

    /// The title of the stanza.
    ///
    /// In the input text file, a stanza is preceeded by a single line, that is assumed to be the
    /// title.
    pub title: Option<String>,
}

impl<'a> Stanza<'a> {
    fn new() -> Stanza<'a> {
        Stanza {
            lines: vec![],
            title: None,
        }
    }

    /// Generates a summary of the stanza and its analysis in a text format.
    ///
    /// The summary includes the raw text and information about each word/token
    /// in each line. The format is targeted for printing to a terminal or put in a
    /// `<pre>` html block.
    pub fn summarize_to_text(&self) -> String {
        let mut out = String::with_capacity(8192); // Arbitrary.

        if let Some(title) = &self.title {
            out.push_str(&format!("TITLE: {}\n", &title));
        }

        for line in &self.lines {
            out.push_str(&line.raw_text);
            out.push('\n');
            for token in &line.tokens {
                if let Some(entry) = &token.entry {
                    out.push_str(&format!("\t{}: {}\n", &token.text, &entry));
                } else {
                    out.push_str(&format!("\t{}: None.\n", &token.text));
                }
            }
            out.push_str(&format!(
                "\t==> Line summary: {} syllables.\n",
                line.num_syllables()
            ));
            out.push('\n');
        }
        if self.has_unknown_words() {
            out.push_str(&format!(
                "Warning: The text has some unknown words. Analysis may suffer.\n"
            ));
        }
        return out;
    }

    /// Returns the number of lines in the Stanza.
    pub fn num_lines(&self) -> usize {
        self.lines.len()
    }

    /// Returns whether there are any unkonwn words across the entire stanza.
    pub fn has_unknown_words(&self) -> bool {
        self.lines.iter().any(|l| l.has_unknown_words())
    }

    /// Returns `StanzaView`s for all possible interpretations of the `Stanza`.
    ///
    /// This is the cartesean product of all the Tokens that have more than one
    /// `Entry` set for them.
    pub fn interpretations(&self) -> InterpretationsIter {
        InterpretationsIter::new(self)
    }
}

/// A view into a `Stanza` that has at most one Entry per word.
///
/// As there can be many different pronunciations of words in a Stanza, and several possible
/// dictionary words for each spot, a Stanza may have several different "interpretations."
///
/// When it is time to analyze a Stanza, all of the Stanza's possible Interpretations are analyzed
/// to see if any of them are correct (or match a pattern, etc.).
#[derive(Clone)]
pub struct StanzaView<'a> {
    stanza: &'a Stanza<'a>,
    lines: Vec<LineView<'a>>,
}

impl<'a> StanzaView<'a> {
    /// Creates a View over the given Stanza.
    fn new(s: &'a Stanza) -> StanzaView<'a> {
        let mut result = StanzaView {
            stanza: s,
            lines: Vec::with_capacity(s.lines.len()),
        };
        for line in &s.lines {
            result.lines.push(LineView::new(&line));
        }
        return result;
    }

    /// Returns the number of lines in the `StanzaView` (and corresponding `Stanza`).
    pub fn num_lines(&self) -> usize {
        self.lines.len()
    }
}

impl<'a> std::fmt::Debug for StanzaView<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self.lines)
    }
}

/// Presents a view of a `Line` in a `Stanza` where there is at most one `Entry` per word.
#[derive(Clone, Debug)]
pub struct LineView<'a> {
    line: &'a Line<'a>,
}

impl<'a> LineView<'a> {
    /// Initializes a view referring to the given line.
    fn new(l: &'a Line) -> LineView<'a> {
        LineView { line: l }
    }

    /// Returns the `Entry` for the `idx`-th token on the line.
    pub fn get_entry(&self, idx: usize) -> Option<&Entry> {
        if let Some(e) = self.line.tokens[idx].entry {
            Some(e)
        } else {
            None
        }
    }

    /// Returns the `Entry` for the last token on the line.
    ///
    /// This is a convenience function to help with rhyming.
    pub fn last_entry(&self) -> Option<&Entry> {
        self.get_entry(self.line.tokens.len() - 1)
    }

    /// Returns the number of syllables in the line.
    ///
    /// This will be an underestimate if `has_unknown_words()`.
    pub fn num_syllables(&self) -> i32 {
        let mut num_syllables = 0;
        for tok in &self.line.tokens {
            if let Some(entry) = tok.entry {
                num_syllables += entry.num_syllables();
            }
        }
        return num_syllables;
    }

    /// Returns the line number from the original snippet corresponding to this Line.
    pub fn num(&self) -> usize {
        self.line.num
    }

    /// Returns whether there are any words on this line that aren't in the dictionary.
    pub fn has_unknown_words(&self) -> bool {
        self.line
            .tokens
            .iter()
            .filter(|x| x.entry.is_none())
            .count()
            > 0
    }
}

/// Generates / iterates over all possible interpretations of a `Stanza`.
///
/// NOTE: Currently this will only produce one `StanzaView` because the dictionary does not return
/// multiple variants per word.
pub struct InterpretationsIter<'a> {
    view: StanzaView<'a>,
    first_run: bool,
}

impl<'a> InterpretationsIter<'a> {
    /// Creates an iterator over all interpretations of the given Stanza.
    ///
    /// The Stanza must outlive this iterator.
    fn new(s: &'a Stanza) -> Self {
        Self {
            view: StanzaView::new(s),
            first_run: true,
        }
    }
}

impl<'a> Iterator for InterpretationsIter<'a> {
    type Item = StanzaView<'a>;

    fn next(&mut self) -> Option<StanzaView<'a>> {
        if self.first_run {
            self.first_run = false;
            return Some(self.view.clone());
        }
        return None;
    }
}

/// Returns whether the given Stanza is probably a Haiku.
///
/// If all the words are known, the result will be accurate. If some are unknown
/// and they could possibly fill out lines that are otherwise too short, this
/// function assumes the Stanza is valid.
///
/// Returns:
/// - `Ok(())` if valid.
/// - `Err(info)` if not valid, with a reason why.
fn is_haiku(stanza: &StanzaView) -> Result<(), String> {
    check_stanza_has_num_lines(stanza, 3)?;

    let mut errors = String::new();
    for (i, expected_syllables) in [5, 7, 5].iter().enumerate() {
        if let Err(info) = check_line_has_num_syllables(&stanza.lines[i], *expected_syllables) {
            errors.push_str(&info);
        }
    }
    if errors.is_empty() {
        return Ok(());
    } else {
        return Err(errors);
    }
}

/// Returns whether the given Stanza is probably a Shakespearean Sonnet.
///
/// If all the words are known, the result will be accurate. If some are unknown
/// and they could possibly fill out lines that are otherwise too short, this
/// function assumes the Stanza is valid.
///
/// Returns:
/// - `Ok(())` if valid.
/// - `Err(info)` if not valid, with a reason why.
fn is_shakespearean_sonnet(stanza: &StanzaView) -> Result<(), String> {
    check_stanza_has_num_lines(stanza, 14)?;

    let mut errors = String::new();
    for line in &stanza.lines {
        if let Err(info) = check_line_has_num_syllables(&line, 10) {
            errors.push_str(&info);
        }
    }
    let rhyming_lines = [(0, 2), (1, 3), (4, 6), (5, 7), (8, 10), (9, 11), (12, 13)];
    for (a, b) in rhyming_lines {
        if let Err(info) = check_lines_rhyme(&stanza.lines[a], &stanza.lines[b]) {
            errors.push_str(&info);
        }
    }

    if errors.is_empty() {
        return Ok(());
    } else {
        return Err(errors);
    }
}

/// Checks that the two given lines rhyme.
///
/// Rhyming is currently that they share the same last syllable. This is conservative and treats
/// unknown words as correct.
fn check_lines_rhyme(a: &LineView, b: &LineView) -> Result<(), String> {
    let a_last_entry = a.last_entry();
    let b_last_entry = b.last_entry();
    if a_last_entry.is_none() || b_last_entry.is_none() {
        return Ok(());
    }

    if a_last_entry.unwrap().rhymes_with(&b_last_entry.unwrap()) {
        Ok(())
    } else {
        Err(format!(
            "lines {} and {}: the words {} and {} don't rhyme?\n",
            a.num(),
            b.num(),
            &a_last_entry.unwrap(),
            &b_last_entry.unwrap()
        ))
    }
}

/// Checks that the Stanza has exactly the given number of lines.
///
/// Returns:
/// - `Ok(())` if valid.
/// - `Err(info)` if not valid, with a reason why.
fn check_stanza_has_num_lines(stanza: &StanzaView, n: usize) -> Result<(), String> {
    if stanza.num_lines() != n {
        return Err(format!(
            "Expected {} lines but the stanza has {}.\n",
            n,
            stanza.num_lines()
        ));
    }
    Ok(())
}

/// Checks that the given Line has the given number of syllables.
///
/// This is conservative in the face of unknown words. If some words are unknown
/// and the number of syllables is short of the target, it will assume that the
/// line is valid.
///
/// Returns:
/// - `Ok(())` if valid.
/// - `Err(info)` if not valid, with reason why.
fn check_line_has_num_syllables(line: &LineView, expected: i32) -> Result<(), String> {
    let mut errors = String::new();

    let num_syllables = line.num_syllables();
    if line.has_unknown_words() {
        if num_syllables >= expected {
            errors.push_str(&format!(
                "line {} has unknown words and {} syllables already, so it will exceed \
                    the limit of {}. \n",
                line.num(),
                num_syllables,
                expected
            ));
        }
        // Assume that the line is ok.
    } else if num_syllables != expected {
        errors.push_str(&format!(
            "line {} has {} syllables but should have {}. \n",
            line.num(),
            num_syllables,
            expected
        ));
    }
    if errors.is_empty() {
        return Ok(());
    } else {
        return Err(errors);
    }
}

/// Finds and analyzes all the stanzas in the given string.
///
/// Stanzas must have more than one line, and they are separated by one or more
/// blank lines. If a Stanza is preceeded by a single line, that is used as the
/// Stanza's title.
///
/// ```raw
/// A duck walked the streets
/// Searching for crumbs of crackers
/// Quacking constantly
///
/// Valentine
///
/// Roses are red
/// Violets are blue
/// Sugar is sweet
/// And so are you
/// ```
///
/// This parses as two stanzas, the first without a title, and the second with
/// the title "Valentine".
///
/// Arguments:
/// * `input` - some raw input, like the contents of a file or a field from a form
/// * `dict` - the Dictionary to use for word lookups
pub fn get_stanzas_from_text<'a>(input: &str, dict: &'a Dictionary) -> Vec<Stanza<'a>> {
    let mut output = vec![];
    let mut stanza = Stanza::new();

    // The candidate_title is filled in whenever there is a stanza with only one line.
    // This is saved so that, if it is followed by a stanza with more than one line, it
    // will be used as the title.
    let mut candidate_title: Option<String> = None;

    for (line_num, raw_line) in input.lines().enumerate() {
        let line = raw_line.trim();
        // Skip comment lines.
        if line.starts_with('#') {
            continue;
        }

        // Finalize the current stanza at each new empty line.
        if line.is_empty() {
            match stanza.num_lines() {
                0 => continue,
                1 => {
                    // Treat this as a possible title.
                    candidate_title = Some(stanza.lines[0].raw_text.clone());
                    stanza = Stanza::new();
                    continue;
                }
                _ => {
                    // Valid Stanza, so keep it.
                    stanza.title = candidate_title; // Could be None. That's ok.
                    candidate_title = None;
                    output.push(stanza);
                    stanza = Stanza::new();
                }
            }
            continue;
        }
        stanza.lines.push(Line::new_from_line(line, line_num, dict));
    }
    // Finalize last stanza.
    if stanza.num_lines() >= 2 {
        stanza.title = candidate_title; // Could be None. That's ok.
        output.push(stanza);
    }
    return output;
}

/// Analyzes the file at `path`, printing the results to the terminal.
///
/// # Arguments
///
/// * `path` - The text file to analyze.
/// * `dict` - The dictionary to use.
///
pub fn analyze_one_file_to_terminal(path: &str, dict: &Dictionary) {
    let raw_input = std::fs::read_to_string(path).unwrap();
    let stanzas = get_stanzas_from_text(&raw_input, dict);
    for s in stanzas {
        println!("====== STANZA ======\n{}", s.summarize_to_text());

        for i in s.interpretations() {
            if i.num_lines() == 14 {
                match is_shakespearean_sonnet(&i) {
                    Ok(_) => println!("This is a valid Shakespearean Sonnet!"),
                    Err(txt) => println!("This isn't a Shakespearean Sonnet because:\n{}\n", &txt),
                }
            }
            if is_haiku(&i).is_ok() {
                println!("What a great haiku!\n\n");
            }
        }
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
pub fn normalize_for_lookup(term: &str) -> String {
    let lowercased_term = term.to_lowercase(); // N.B.: This could be folded into the loop below.

    // Strip a whole bunch of unnecessary punctuation, and search for a couple of interesting
    // cases of punctuation in the middle that may need special handling.
    //
    // Detect cases like "A.M.". If there is a period in the middle of the term
    // somewhere, then the periods won't be stripped below.
    let mut found_period = false;
    let mut has_inner_periods = false;
    let mut result: String = lowercased_term
        .chars()
        .filter_map(|c| match c {
            '!' | ',' | '?' | ':' | ';' | '"' | '“' | '”' => None,
            '’' => Some('\''), // Switch the curly apostrophe to the ASCII verison.
            '.' => {
                found_period = true;
                Some(c) // Stripping these is conditional and done below.
            }
            _ => {
                if found_period {
                    // Assume that any character not listed above is valid for the dictionary,
                    // in which case the current character indicates that the word is
                    // continuing, so the period that was found before wasn't at the end.
                    has_inner_periods = true;
                }
                Some(c)
            }
        })
        .collect();

    // This case mostly catches the ends of sentences and words with ellipses...
    if !has_inner_periods {
        result = result.replace('.', "");
    }

    // This is a way of doing str::trim_end_matches('-') in-place.
    while result.ends_with('-') {
        result.pop();
    }

    return result;
}

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
        assert_eq!(normalize_for_lookup("cure:"), "cure");
        assert_eq!(normalize_for_lookup("ground;"), "ground");
        assert_eq!(normalize_for_lookup("\"Nope.\""), "nope");

        // Curly quotes should be treated as the ascii equivalents.
        assert_eq!(normalize_for_lookup("“double”"), "double");
        assert_eq!(normalize_for_lookup("single’s"), "single's");

        // For now, only trailing dashes should be removed.
        assert_eq!(normalize_for_lookup("pen--"), "pen");
        assert_eq!(normalize_for_lookup("well-contented"), "well-contented");

        // Periods should be preserved if they also appear within the term.
        assert_eq!(normalize_for_lookup("A.M."), "a.m.");
        assert_eq!(normalize_for_lookup("p.m.,"), "p.m.");
    }

    #[test]
    fn test_get_stanzas_from_text_selects_correct_text_blocks() {
        let dict = Dictionary::new(); // Empty is ok, not testing lookups here.
        let input = "\
             # Some comment to be ignored.\n\
             A duck walked the streets\n\
             Searching for crumbs of crackers\n\
             Quacking constantly\n\
             \n\
             \n\
             This single empty line should be ignored / replaced by the next.\n\
             \n\
             Valentine\n\
             \n\
             Roses are red\n\
             Violets are blue\n\
             Sugar is sweet\n\
             And so are you\n";
        let output = get_stanzas_from_text(&input, &dict);
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].title, None);
        assert_eq!(output[0].lines.len(), 3);
        assert_eq!(output[1].title.as_ref().unwrap(), "Valentine");
        assert_eq!(output[1].lines.len(), 4);
    }

    mod is_haiku {
        use super::*;

        #[test]
        fn test_with_valid_known_words() {
            // Valid haikus with all words in the dictionary should be valid.
            let test_dictionary = vec![
                "a AH0",
                "a(2) EY1",
                "bits B IH1 T S",
                "constantly K AA1 N S T AH0 N T L IY0",
                "crackers K R AE1 K ER0",
                "crumbs K R AH1 M Z",
                "duck D AH1 K",
                "for F AO1 R",
                "for(2) F ER0",
                "for(3) F R ER0",
                "of AH1 V",
                "quacking K W AE1 K IH0 NG",
                "searching S ER1 CH IH0 NG",
                "streets S T R IY1 T S",
                "the DH AH0",
                "the(2) DH AH1",
                "the(3) DH IY0",
                "walked W AO1 K T",
            ];
            let mut dict = Dictionary::new();
            dict.insert_all(&test_dictionary);

            let text = "\
              A duck walked the streets\n\
              Searching for crumbs of crackers\n\
              Quacking constantly\n";
            let stanza = to_stanza(text, &dict);
            assert!(!stanza.has_unknown_words()); // Test invariant.
            assert_eq!(is_haiku(&unique_interp(&stanza)), Ok(()));
        }

        #[test]
        fn test_checks_exact_syllable_counts_with_known_words() {
            let test_dictionary = vec!["a AH0"];
            let mut dict = Dictionary::new();
            dict.insert_all(&test_dictionary);

            // If all the words are known and the number of syllables are not correct,
            // the stanza should not be valid.
            let line1_too_short = "a a a a\na a a a a a a\na a a a a";
            let line2_too_short = "a a a a a\na a a a a a\na a a a a";
            let line3_too_short = "a a a a a\na a a a a a a\na a a a";
            let all_too_long = "a a a a a a\na a a a a a a a\na a a a a a";
            let stanza = to_stanza(&line1_too_short, &dict);
            assert!(!stanza.has_unknown_words()); // Test invariant.
            assert!(is_haiku(&unique_interp(&stanza)).is_err());

            let stanza = to_stanza(&line2_too_short, &dict);
            assert!(is_haiku(&unique_interp(&stanza)).is_err());

            let stanza = to_stanza(&line3_too_short, &dict);
            assert!(is_haiku(&unique_interp(&stanza)).is_err());

            let stanza = to_stanza(&all_too_long, &dict);
            assert!(is_haiku(&unique_interp(&stanza)).is_err());
        }

        #[test]
        fn test_requires_exactly_3_lines() {
            let test_dictionary = vec!["a AH0"];
            let mut dict = Dictionary::new();
            dict.insert_all(&test_dictionary);

            let two_lines = "a a a a a\na a a a a a a";
            let four_lines = "a a a a a\na a a a a a a\na a a a a\na a a a a";

            let stanza = to_stanza(&two_lines, &dict);
            assert!(is_haiku(&unique_interp(&stanza)).is_err());

            let stanza = to_stanza(&four_lines, &dict);
            assert!(is_haiku(&unique_interp(&stanza)).is_err());
        }

        #[test]
        fn test_is_conservative_with_unknown_words() {
            let test_dictionary = vec!["a AH0"];
            let mut dict = Dictionary::new();
            dict.insert_all(&test_dictionary);

            let text = "a a a a someword\n\
                        wertgreen\n\
                        a a a a a";
            let stanza = to_stanza(&text, &dict);
            assert!(is_haiku(&unique_interp(&stanza)).is_ok());
        }

        #[test]
        fn test_fails_with_unknown_words_when_clearly_too_long() {
            let test_dictionary = vec!["a AH0"];
            let mut dict = Dictionary::new();
            dict.insert_all(&test_dictionary);

            let text = "a a a a a toolong\n\
                        a a a a a a a\n\
                        a a a a a";
            let stanza = to_stanza(&text, &dict);
            assert!(is_haiku(&unique_interp(&stanza)).is_err());
        }
    }

    /// This test helper parses `text` to extract exactly one `Stanza`.
    ///
    /// It requires there is exactly one in the input.
    fn to_stanza<'a>(text: &str, dict: &'a Dictionary) -> Stanza<'a> {
        let mut stanzas = get_stanzas_from_text(&text, &dict);
        assert_eq!(stanzas.len(), 1);
        stanzas.pop().unwrap()
    }

    /// This test helper extracts interpretations from `Stanza` and expects only one.
    fn unique_interp<'a>(stanza: &'a Stanza) -> StanzaView<'a> {
        let mut iter = stanza.interpretations();
        return iter.next().unwrap();
    }

    mod is_shakespearean_sonnet {
        use super::*;

        #[test]
        fn test_requires_14_lines() {
            let test_dictionary = vec!["a AH0"];
            let mut dict = Dictionary::new();
            dict.insert_all(&test_dictionary);

            let line = "a a a a a a a a a a\n";
            let too_short = to_stanza(&line.repeat(13), &dict);
            assert!(is_shakespearean_sonnet(&unique_interp(&too_short)).is_err());

            let correct = to_stanza(&line.repeat(14), &dict);
            assert!(is_shakespearean_sonnet(&unique_interp(&correct)).is_ok());

            let too_long = to_stanza(&line.repeat(15), &dict);
            assert!(is_shakespearean_sonnet(&unique_interp(&too_long)).is_err());
        }

        #[test]
        fn test_lines_must_have_10_syllables() {
            let test_dictionary = vec!["a AH0"];
            let mut dict = Dictionary::new();
            dict.insert_all(&test_dictionary);

            let full_line = "a a a a a a a a a a\n";
            let short_line = "a a a a a a a a a\n";
            let long_line = "a a a a a a a a a a a\n";

            // Just test inserting a short or long line in two spots in the middle.
            // This is cheating a bit with coverage but a lot is covered with the
            // Haiku tests already, including unknown word handling.
            let prose_short = format!(
                "{}{}{}",
                &full_line.repeat(8),
                &short_line,
                &full_line.repeat(5)
            );
            let prose_long = format!(
                "{}{}{}",
                &full_line.repeat(3),
                &long_line,
                &full_line.repeat(10)
            );

            let too_short = to_stanza(&prose_short, &dict);
            assert!(is_shakespearean_sonnet(&unique_interp(&too_short)).is_err());
            let too_long = to_stanza(&prose_long, &dict);
            assert!(is_shakespearean_sonnet(&unique_interp(&too_long)).is_err());
        }

        #[test]
        fn test_rhyming_pattern() {
            // Sonnet lines should match the rhyming pattern ABAB CDCD EFEF GG.
            let poem_dict_entries = vec![
                // These are the words from the original poem.
                "aloud AH0 L AW1 D",
                "apart AH0 P AA1 R T",
                "crowd K R AW1 D",
                "ferns F ER1 N Z",
                "gazelle G AH0 Z EH1 L",
                "give G IH1 V",
                "group G R UW1 P",
                "live L IH1 V", // live(2) in cmudict.
                "poop P UW1 P",
                "smart S M AA1 R T",
                "spiteful S P AY1 T F AH0 L",
                "terns T ER1 N Z",
                "unwell AH0 N W EH1 L",
                "unimpressible AH2 N IH2 M P R EH1 S AH0 B AH0 L",
                // These are some other words that don't rhyme with those.
                "one W AH1 N",
                "zebra Z IY1 B R AH0", // 2 syllables.
                "electroplating IH2 L EH1 K T R AH0 P L EY2 T IH0 NG", // 5 syllables.
            ];
            let mut poem_dict = Dictionary::new();
            poem_dict.insert_all(&poem_dict_entries);

            let poem = "\
                A tricksy girl named Stella once did live\n\
                among acacia thorns and ng’ombe poop.\n\
                In Tanzanian hinterlands rains give\n\
                sweet life to every possible food group.\n\
                But in this verdant, fertile land of ferns,\n\
                inhabited by bustards, storks, gazelle,\n\
                and lions, warthogs, bees, gnats, buzzards, terns,\n\
                our dearest Stella found herself unwell.\n\
                She’d hoped to have made friends, but she was smart.\n\
                Schoolkids at her poked fun, in jests spiteful.\n\
                Performance set her in a class apart\n\
                that left her classmates unimpressible.\n\
                Post-class one day, and distanced from the crowd,\n\
                our Stella pondered friendlessness aloud.\n";

            {
                // Original, correct version shoudld be ok.
                let stanza = to_stanza(&poem, &poem_dict);
                assert!(is_shakespearean_sonnet(&unique_interp(&stanza)).is_ok());
            }

            // For the last word in each line, replace it with another word of the same length
            // that doesn't rhyme, and verify that the test fails.
            let replacements = [
                ("live", "one"),
                ("poop", "one"),
                ("give", "one"),
                ("group", "one"),
                ("ferns", "one"),
                ("gazelle", "zebra"),
                ("terns", "one"),
                ("unwell", "zebra"),
                ("smart", "one"),
                ("spiteful", "zebra"),
                ("apart", "zebra"),
                ("unimpressible", "electroplating"),
                ("crowd", "one"),
                ("aloud", "zebra"),
            ];
            for (old, new) in replacements {
                let text = poem.replace(old, new);
                let stanza = to_stanza(&text, &poem_dict);
                assert!(is_shakespearean_sonnet(&unique_interp(&stanza)).is_err());
            }
        }
    } // mod is_shakespearean_sonnet
}
