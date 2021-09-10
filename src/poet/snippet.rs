// A "snippet" refers to any bit of text to be analyzed. It is most often a
// poem or other prose, but it can also be a single word.
//
// Generally, the text in a snippet will have punctuation, capitalization,
// and other formatting that may have to be removed, and it often also has
// intentional structure (e.g. the breaking of words into lines) that is
// important.


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
    const PUNCTUATION_TO_STRIP: &[char] = &['.', '!', ',', '?'];
    // This makes a copy of the string but remove_matches() is an experimental api still.
    if has_inner_periods {
        result = result.replace(&PUNCTUATION_TO_STRIP[1..], "");
    } else {
        result = result.replace(&PUNCTUATION_TO_STRIP[..], "");
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

        // Periods should be preserved if they also appear within the term.
        assert_eq!(normalize_for_lookup("A.M."), "a.m.");
        assert_eq!(normalize_for_lookup("p.m.,"), "p.m.");
    }
}

