use std::fs::File;
use std::io::{BufRead, BufReader};

// A "snippet" refers to any bit of text to be analyzed. It is most often a
// poem or other prose, but it can also be a single word.
//
// Generally, the text in a snippet will have punctuation, capitalization,
// and other formatting that may have to be removed, and it often also has
// intentional structure (e.g. the breaking of words into lines) that is
// important.

use crate::poet::dictionary::*;

/// Analyzes the file at `path`, printing the results to the terminal.
///
/// # Arguments
///
/// * `path` - The text file to analyze.
/// * `dict` - The dictionary to use.
///
pub fn analyze_one_file_to_terminal(path: &str, dict: &Dictionary) {
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
    const PUNCTUATION_TO_STRIP: &[char] = &['.', '!', ',', '?', ':', ';'];
    // This makes a copy of the string but remove_matches() is an experimental api still.
    if has_inner_periods {
        result = result.replace(&PUNCTUATION_TO_STRIP[1..], "");
    } else {
        result = result.replace(&PUNCTUATION_TO_STRIP[..], "");
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

        // For now, only trailing dashes should be removed.
        assert_eq!(normalize_for_lookup("pen--"), "pen");
        assert_eq!(normalize_for_lookup("well-contented"), "well-contented");

        // Periods should be preserved if they also appear within the term.
        assert_eq!(normalize_for_lookup("A.M."), "a.m.");
        assert_eq!(normalize_for_lookup("p.m.,"), "p.m.");
    }
}

