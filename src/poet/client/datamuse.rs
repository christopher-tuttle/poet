//! An RPC client for the Datamuse / RhymeZone API.
//!
//! The Datamuse API powers RhymeZone and several other sites, and it provides many different
//! word-finding capabilities, including spell correction, suggestion, context clues,
//! and semantic meaning.
//!
//! See <https://www.datamuse.com/api/>.

use rocket::serde::Deserialize;
use url::Url;

use crate::poet::*;

/// A Datamuse API client.
pub struct Client {}

/// A builder / helper for constructing Datamuse API requests.
///
/// # Example Usage
///
/// ```
/// let url = UrlBuilder::new().sounds_like("chicken").max(10).build();
/// ```
struct UrlBuilder {
    /// The "Sounds Like" / `sl=` URL param.
    sounds_like: Option<String>,

    /// The "Spelled Like" / `sp=` URL param.
    spelled_like: Option<String>,

    /// Whether to use the "query echo" / `qe=` URL param.
    ///
    /// If enabled, exactly one of sounds_like or spelled_like must be provided, and
    /// max will be set to 1.
    query_echo: bool,

    /// The maximum number of results to return.
    ///
    /// The API has default 100, max 1000.
    max: Option<usize>,

    /// Metadata Flag 's': Whether to return an estimate on the number of syllables.
    want_syllable_count: bool,

    /// Metadata Flag 'r': Whether to return a best-guess pronunciation.
    ///
    /// Default `true`, because it's common right now.
    want_pronunciation: bool,
}

impl UrlBuilder {
    /// Creates a new builder. Callers should specify at least one of `sounds_like()` or
    /// `spelled_like()`, set any options, and then finish with `build()`.
    fn new() -> UrlBuilder {
        UrlBuilder {
            sounds_like: None,
            spelled_like: None,
            query_echo: false,
            max: None,
            want_syllable_count: false,
            want_pronunciation: true,
        }
    }

    /// Sets the `sp=` / "Spelled Like" query param:
    ///
    /// Spelled like constraint: require that the results are spelled similarly to this string of
    /// characters, or that they match this wildcard pattern. A pattern can include any combination
    /// of alphanumeric characters, spaces, and two reserved characters that represent placeholders
    /// — `*` (which matches any number of characters) and `?` (which matches exactly one character).
    fn spelled_like(mut self, term: &str) -> Self {
        self.spelled_like = Some(String::from(term));
        self
    }

    /// Sets the `sl=` / "Sounds Like" query param:
    ///
    /// Sounds like constraint: require that the results are pronounced similarly to this string of
    /// characters. (If the string of characters doesn't have a known pronunciation, the system
    /// will make its best guess using a text-to-phonemes algorithm.)
    #[allow(dead_code)]
    fn sounds_like(mut self, term: &str) -> Self {
        self.sounds_like = Some(String::from(term));
        self
    }

    /// Enables "query echo" pointing at either `sl` or `sp`, exactly one of which must be set.
    ///
    /// This enables a kind of exact lookup by spelling or sound:
    ///
    /// Query echo: The presence of this parameter asks the system to prepend a result to the
    /// output that describes the query string from some other parameter, specified as the argument
    /// value. This is useful for looking up metadata about specific words. For example,
    /// `/words?sp=flower&qe=sp&md=fr` can be used to get the pronunciation and word frequency for
    /// `flower`.
    ///
    /// Using this option also sets `max` to `1`, which can be overridden.
    fn query_echo(mut self) -> Self {
        self.query_echo = true;
        self.max = Some(1);
        self
    }

    /// Sets the max number of results to return. REQUIRES: `max <= 1000`.
    #[allow(dead_code)]
    fn max(mut self, max: usize) -> Self {
        debug_assert!(max <= 1000);
        self.max = Some(max);
        self
    }

    /// Requests that the results include an estimate of syllable counts `&md=s`.
    #[allow(dead_code)]
    fn want_syllables(mut self) -> Self {
        self.want_syllable_count = true;
        self
    }

    /// Builds and returns the request URL.
    fn build(&self) -> String {
        let mut url = Url::parse("https://api.datamuse.com/words?").unwrap();

        let mut query_pairs = url.query_pairs_mut();
        if let Some(term) = &self.sounds_like {
            query_pairs.append_pair("sl", &term);
        }
        if let Some(term) = &self.spelled_like {
            query_pairs.append_pair("sp", &term);
        }

        if self.query_echo {
            if self.spelled_like.is_some() {
                debug_assert!(self.sounds_like.is_none());
                query_pairs.append_pair("qe", "sp");
            } else {
                debug_assert!(self.sounds_like.is_some());
                query_pairs.append_pair("qe", "sl");
            }
        }

        if let Some(n) = &self.max {
            query_pairs.append_pair("max", &format!("{}", n));
        }

        let mut md_flags = String::with_capacity(10);
        if self.want_syllable_count {
            md_flags.push('s');
        }
        if self.want_pronunciation {
            md_flags.push('r');
        }
        if md_flags.len() > 0 {
            query_pairs.append_pair("md", &md_flags);
        }

        // To commit the changes to the URL: "The state of Url is unspecified if this return value
        // is leaked without being dropped."
        drop(query_pairs);

        return String::from(url.as_str());
    }
}

/// A `serde`-ready structure mirroring the JSON format for Datamuse's `/words` output.
///
/// Example `/words` respose:
/// ```raw
/// [{"word":"bustards","score":129367,"numSyllables":2,"tags":["pron:B AH1 S T ER0 D Z "]},
///  {"word":"bustard","score":65133,"numSyllables":2,"tags":["pron:B AH1 S T ER0 D "]},
///  ...]
/// ```
///
/// The `pron:` tags have phonemes. Other tags include `f:<float>` for word frequency per
/// million words, and `n`, `v`, `adj`, `adv`, and `u` for parts of speech. Neither of these
/// (`md=p` and `md=f`) are currently requested.
#[allow(non_snake_case)] // Field names have to match the remote JSON API.
#[derive(Debug, Deserialize)]
#[serde(crate = "rocket::serde")]
struct WordsApiItem {
    word: String,
    score: i32,
    numSyllables: Option<usize>,
    tags: Vec<String>,
}

impl WordsApiItem {
    /// Converts this response element to an `Entry`.
    ///
    /// Requires that the item has a `pron:` tag (enabled by default on all requests).
    fn to_entry(&self) -> dictionary::Entry {
        for t in &self.tags {
            if t.starts_with("pron:") {
                // TODO: Check for multiple pronunciations. I haven't seen any in the responses
                // thus far, though it might be good to warn if one appears.
                return dictionary::Entry::from_parts(&self.word, &t[5..]);
            }
        }
        // TODO: This is not ideal, but ok for now.
        return dictionary::Entry::from_parts(&self.word, "");
    }
}

/// An async client for the Datamuse API.
impl Client {
    pub fn new() -> Client {
        Client {}
    }

    /// Fetches the pronunciation of `term` and returns it as an `Entry`.
    ///
    /// If the term is in Datamuse's dictionaries, the phonemes should be correct for at least one
    /// pronunciation of the term. If not, the response will be the best guess from Datamuse of the
    /// term's phonemes.
    pub async fn get_phonemes(
        &self,
        term: &str,
    ) -> Result<Option<dictionary::Entry>, Box<dyn std::error::Error>> {
        let url = UrlBuilder::new().spelled_like(term).query_echo().build();
        let resp = self.do_words_request(&url).await?;

        if resp.is_empty() {
            return Ok(None);
        }

        if resp.len() != 1 {
            debug!("Unexpected / extra results for {} : {:?}", &url, &resp);
        }

        let r = &resp[0];
        if &r.word != term {
            debug!(
                "Unexpected word {} found as first result when looking up {}.",
                &r.word, term
            );
        }
        return Ok(Some(r.to_entry()));
    }

    /// Issues a request to `url` and parses the result as a `/words` api call.
    async fn do_words_request(
        &self,
        url: &str,
    ) -> Result<Vec<WordsApiItem>, Box<dyn std::error::Error>> {
        print!("fetching {}...", &url);
        let resp = reqwest::get(url).await?.json::<Vec<WordsApiItem>>().await?;
        println!("done.");
        return Ok(resp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_builder() {
        // Basic cases of query words: make sure they're urlencoded.
        assert_eq!(
            UrlBuilder::new().spelled_like("plz&escape me").build(),
            "https://api.datamuse.com/words?sp=plz%26escape+me&md=r"
        );
        assert_eq!(
            UrlBuilder::new().sounds_like("escape/me&too?").build(),
            "https://api.datamuse.com/words?sl=escape%2Fme%26too%3F&md=r"
        );

        // Test with the various options.
        assert_eq!(
            UrlBuilder::new()
                .sounds_like("a")
                .spelled_like("b")
                .max(42)
                .want_syllables()
                .build(),
            "https://api.datamuse.com/words?sl=a&sp=b&max=42&md=sr"
        );

        // Testing with Query Echo.
        assert_eq!(
            UrlBuilder::new().sounds_like("flower").query_echo().build(),
            "https://api.datamuse.com/words?sl=flower&qe=sl&max=1&md=r"
        );
        assert_eq!(
            UrlBuilder::new()
                .spelled_like("flower")
                .query_echo()
                .max(42)
                .build(),
            "https://api.datamuse.com/words?sp=flower&qe=sp&max=42&md=r"
        );
    }

    #[test]
    fn word_api_to_entry() {
        let input = WordsApiItem {
            word: String::from("flowers"),
            score: 129846,
            numSyllables: Some(2),
            tags: vec![
                String::from("pron:F L AW1 ER0 Z "),
                String::from("f:51.905237"),
            ],
        };
        let entry = input.to_entry();
        assert_eq!(entry.word, "flowers");
        assert_eq!(entry.variant, 1);
        assert_eq!(entry.phonemes.phonemes, vec!["F", "L", "AW1", "ER0", "Z"]);
        assert_eq!(entry.phonemes.num_syllables(), 2);
    }
}
