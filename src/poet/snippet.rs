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

    /// The tokenized version of `raw_text`.
    pub tokens: Vec<Token<'a>>,
}

impl<'a> Line<'a> {
    /// Analyzes the given line and produces a new `Line` with the results.
    ///
    /// Arguments:
    /// * `raw` - A non-empty line from a poem, e.g. `Roses are red,`.
    /// * `dict` - The dictionary to use for word lookups.
    ///
    /// Lifetime note: the `Dictionary` must outlive the returned `Line`.
    fn new_from_line<'b>(raw: &str, dict: &'b Dictionary) -> Line<'b> {
        let mut result = Line {
            raw_text: raw.to_string(),
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
                num_syllables += entry.syllables;
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
    pub lines: Vec<Line<'a>>,
}

impl<'a> Stanza<'a> {
    fn new() -> Stanza<'a> {
        Stanza { lines: vec![] }
    }

    /// Generates a summary of the stanza and its analysis in a text format.
    ///
    /// The summary includes the raw text and information about each word/token
    /// in each line. The format is targeted for printing to a terminal or put in a
    /// `<pre>` html block.
    pub fn summarize_to_text(&self) -> String {
        let mut out = String::with_capacity(8192); // Arbitrary.
        for line in &self.lines {
            out.push_str(&line.raw_text);
            out.push('\n');
            for token in &line.tokens {
                if let Some(entry) = &token.entry {
                    out.push_str(&format!("\t{}: {:?}\n", &token.text, &entry));
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
        return out;
    }
}

/// Finds and analyzes all the stanzas in the given string.
///
/// If the input has multiple stanzas/poems in it, then they are assumed to be
/// separated by blank lines.
///
/// Arguments:
/// * `input` - some raw input, like the contents of a file or a field from a form
/// * `dict` - the Dictionary to use for word lookups
///
/// TODO: Document better.
pub fn get_stanzas_from_text<'a>(input: &str, dict: &'a Dictionary) -> Vec<Stanza<'a>> {
    // First attempt: Just create a new stanza each time that there is a blank
    // line after one or more non-blank ones.
    //
    // TODO:
    // Treat single lines as titles for handling multiple-stanza inputs.

    let mut output = vec![];
    let mut stanza = Stanza::new();
    for raw_line in input.lines() {
        let line = raw_line.trim();
        // Finalize the current stanza at each new line.
        if line.is_empty() {
            if !stanza.lines.is_empty() {
                output.push(stanza);
                stanza = Stanza::new();
            }
            continue;
        }
        stanza.lines.push(Line::new_from_line(line, dict));
    }
    // Finalize last stanza.
    if !stanza.lines.is_empty() {
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
}
