//! HTTP server components for poet.

use rocket::form::Form;
use rocket::serde::Serialize;
use rocket::State;
use rocket_dyn_templates::Template;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::poet::*;

/// A container for data owned by web server that's available for all requests.
struct ServerState {
    shelf: Mutex<dictionary::Shelf>,
}

/// A template-oriented version of SimilarWord.
#[derive(Clone, Debug, Serialize)]
#[serde(crate = "rocket::serde")]
struct SimilarWordTemplateData {
    /// The word.
    word: String,

    /// The number of syllables in `word`.
    syllables: i32,

    /// Larger scores represent higher similarity.
    score: i32,

    /// Pre-serialized phoneme sequence, e.g. "HH AH0 L OW1".
    phonemes: String,
}

/// A data container for populating the lookup template.
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct LookupTemplateData<'a> {
    /// The word being looked up.
    query: &'a str,

    /// Metadata about the query word;
    entry_info: Option<String>,

    /// The total number of results found.
    num_found: usize,

    /// The number of results returned.
    num_returned: usize,

    /// All of the words, sorted in decreasing order of similarity.
    similar_words: Vec<SimilarWordTemplateData>,
}

/// Handler for querying the dictionary for a single term.
#[get("/lookup?<term>&<num>")]
fn lookup(state: &State<ServerState>, term: &str, num: Option<usize>) -> Template {
    let mut data = LookupTemplateData {
        query: term,
        entry_info: None,
        num_found: 0,
        num_returned: 0,
        similar_words: vec![],
    };
    let max_results = num.unwrap_or(500);

    let shelf = state.shelf.lock().unwrap();
    let dict = shelf.over_all();

    if let Some(v) = dict.lookup(term) {
        if v.len() > 1 {
            // FIXME: Ignoring this case.
            println!(
                "Warning: {} has multiple results and only rendering the first.",
                term
            );
        }
        let entry = &v[0];
        data.entry_info = Some(format!("{:?}", entry));
        let similar_result = dict.similar(term);
        data.num_found = similar_result.words.len();
        for word in similar_result.words.into_iter().take(max_results) {
            let word_for_template = SimilarWordTemplateData {
                word: word.word,
                syllables: word.syllables,
                score: word.score,
                phonemes: format!("{}", &word.phonemes),
            };
            data.similar_words.push(word_for_template);
        }
        data.num_returned = data.similar_words.len();
    }
    return Template::render("lookup", data);
}

/// Handler for AJAX lookup of a term (`/api/lookup?term=<query>`).
///
/// The templating is all done server-side at the moment, so this endpoint returns HTML to
/// be inserted into the page.
#[get("/api/lookup?<term>")]
fn api_lookup(state: &State<ServerState>, term: &str) -> String {
    let shelf = state.shelf.lock().unwrap();

    let dict = shelf.over_all();

    if let Some(v) = dict.lookup(term) {
        if v.len() > 1 {
            // FIXME: Ignoring this case.
            println!(
                "Warning: {} has multiple results and only returning one.",
                term
            );
        }
        let entry = &v[0];
        let similar_info = dict.similar(term);
        let num_similar = similar_info.words.len();

        let mut examples = String::with_capacity(1024); // Arbitrary.
        const NUM_WORDS_TO_SHOW: usize = 8;
        for (i, word_info) in similar_info.words.iter().enumerate() {
            if i > NUM_WORDS_TO_SHOW {
                examples.push_str("...");
                break;
            } else {
                examples.push_str(&format!("<b>{}</b>, ", &word_info.word));
            }
        }

        return format!(
            "{} (<code>{}</code>) [{} syllables] with {} similar words like:<br>{}",
            term,
            entry.phonemes,
            entry.num_syllables(),
            num_similar,
            examples
        );
    } else {
        return format!("<em>{}</em> not found.", term);
    }
}

/// Describes the parameters and types for /analyze POST requests.
///
/// This is used by Rocket to validate incoming requests and to pass the values to
/// the `analyze` handler.
#[derive(FromForm)]
struct AnalyzeRequest<'a> {
    /// The text snippet to analyze.
    text: &'a str,
}

/// Handler for a POST form to analyze a block of prose / snippet.
#[post("/analyze", data = "<req>")]
fn analyze(state: &State<ServerState>, req: Form<AnalyzeRequest>) -> Template {
    let mut context = rocket_dyn_templates::tera::Context::new();

    // Copy the user input to the output to pre-fill the form box.
    context.insert("user_input", &req.text);

    let shelf = state.shelf.lock().unwrap();
    let dict = shelf.over_all();

    // Parse the input and break it into one or more stanzas.
    let stanzas = snippet::get_stanzas_from_text(&req.text, dict);

    // Hack: Most of the page is rendered with raw HTML and strings, not with templates.
    // TODO: Figure out how to do sub-templates with Rocket, etc. and fix this.
    let mut html = String::with_capacity(32768); // An arbitrary, "biggish" starting point.

    let mut unknown_words = vec![];

    for stanza in &stanzas {
        unknown_words.append(&mut stanza.unknown_words());

        let best_interpretation = stanza.analyze();

        // This prints the original text of the stanza, the phonemes of each word, the
        // classification, and any errors/warnings from the analysis.
        best_interpretation.append_html_to(&mut html);

        // This prints original text of the stanza, along with every word variant and their
        // phonemes, so that users can see where the analysis may have been incorrect.
        stanza.append_html_to(&mut html);
    }

    context.insert("prose_html", &html);
    context.insert("unknown_words", &unknown_words.join("\n"));
    return Template::render("analyze", context.into_json());
}

/// A trait like `Display` to render various structures as HTML.
///
/// All of these should be done in templates, but this was all written pretty fast and that
/// refactoring hasn't happened yet. So instead, this trait is an experiment to try and write
/// something that's somewhat readable and not also copying big strings all the time.
///
/// It would be cool to use a Formatter to write directly to, but I haven't figured out how to make
/// one yet.
pub trait ToHtml {
    /// Outputs `self` as an HTML string, appended to `out`.
    fn append_html_to(&self, out: &mut String);

    /// Returns an HTML snippet representing `self`.
    ///
    /// This variant is for ease of use when rendering smaller objects inside a `format!` macro.
    /// The default implementation allocates a small `String` to render into.
    fn to_html(&self) -> String {
        let mut out = String::with_capacity(100); // Assumed small-ish if inline.
        self.append_html_to(&mut out);
        out
    }
}

impl ToHtml for snippet::BestInterpretation<'_> {
    /// Renders the interpretation to HTML.
    ///
    /// This includes the `StanzaView` of the best interpretation, the classification, and
    /// any warnings/errors from the analysis.
    fn append_html_to(&self, out: &mut String) {
        let view: &snippet::StanzaView = self.best.as_ref().unwrap();

        out.push_str("<pre>");
        view.append_html_to(out);
        out.push('\n');

        if self.errors.is_empty() {
            out.push_str(&format!("<b>What a great {}!</b>\n", &self.validator));
        } else {
            out.push_str(&format!(
                "This looks like a {}, except for these ...\n",
                &self.validator
            ));
            out.push_str("<span class=\"error_header\">Errors and warnings:</span>\n");
            for e in &self.errors {
                use snippet::ClassifyError::*;
                match &e {
                    StanzaError(_) => {
                        out.push_str(&format!("<span class=\"stanza_warning\">{}</span>\n", &e))
                    }
                    LineError(_, _) => {
                        out.push_str(&format!("<span class=\"line_warning\">{}</span>\n", &e))
                    }
                }
            }
        }
        out.push_str("\n</pre>");
    }
}

impl ToHtml for snippet::Stanza<'_> {
    /// Renders the `Stanza` to HTML.
    ///
    /// This shows the words in the `Stanza` with the corresponding dictionary entries, to aid
    /// in checking the dictionaries have the correct words and interpretations available.
    ///
    /// ```
    /// // https://www.mcsweeneys.net/articles/haiku-a-bitter-duck-might-write
    ///
    /// Humans envy my
    ///         humans: "humans" (HH Y UW1 M AH0 N Z); variant=1, syllables=2
    ///               : "humans" (Y UW1 M AH0 N Z); variant=2, syllables=2
    ///         envy: "envy" (EH1 N V IY0); variant=1, syllables=2
    ///         my: "my" (M AY1); variant=1, syllables=1
    ///
    /// swimming ability. But
    ///         swimming: "swimming" (S W IH1 M IH0 NG); variant=1, syllables=2
    ///         ability: "ability" (AH0 B IH1 L AH0 T IY2); variant=1, syllables=4
    ///         but: "but" (B AH1 T); variant=1, syllables=1
    ///
    /// I wish I could read.
    ///         i: "i" (AY1); variant=1, syllables=1
    ///         wish: "wish" (W IH1 SH); variant=1, syllables=1
    ///         i: "i" (AY1); variant=1, syllables=1
    ///         could: "could" (K UH1 D); variant=1, syllables=1
    ///         read: "read" (R EH1 D); variant=1, syllables=1
    ///             : "read" (R IY1 D); variant=2, syllables=1
    /// ```
    ///
    fn append_html_to(&self, out: &mut String) {
        out.push_str("<pre>");
        if let Some(title) = &self.title {
            out.push_str(&format!("TITLE: {}\n", &title));
        }

        for line in &self.lines {
            out.push_str(&line.raw_text);
            out.push('\n');
            for token in &line.tokens {
                if let Some(entries) = &token.entry {
                    for (i, entry) in entries.iter().enumerate() {
                        if i == 0 {
                            out.push_str(&format!("\t{}: {}\n", &token.text, entry.to_html(),));
                        } else {
                            out.push_str(&format!(
                                "\t{}: {}\n",
                                &" ".repeat(token.text.len()),
                                entry.to_html(),
                            ));
                        }
                    }
                } else {
                    out.push_str(&format!(
                        "\t<span class=\"missing\">{}: not found.</span>\n",
                        &token.text
                    ));
                }
            }
            // TODO: Re-introduce the line summary with the number of syllables.
            out.push('\n');
        }
        if self.has_unknown_words() {
            out.push_str(&format!(
                "Warning: The text has some unknown words. Analysis may suffer.\n"
            ));
        }
        out.push_str("\n</pre>");
    }
}

impl ToHtml for snippet::StanzaView<'_> {
    fn append_html_to(&self, out: &mut String) {
        for line in &self.lines {
            line.append_html_to(out);
            out.push('\n');
        }
    }
}

impl ToHtml for snippet::LineView<'_> {
    /// Renders a `LineView` to explain which words were selected.
    ///
    /// ```
    /// 01. Humans envy my
    ///   . humans............  envy.......  my...  
    ///   . HH Y UW1 M AH0 N Z  EH1 N V IY0  M AY1  
    /// ```
    fn append_html_to(&self, out: &mut String) {
        out.push_str(&format!(
            "{:02} {:2}. {}\n",
            self.num(),
            self.index() + 1,
            self.raw_text(),
        ));
        // Start with just blasting everything there, and then make it pretty / evenly spaced.
        let num_tokens = self.num_words();

        let mut dict_keys: Vec<String> = Vec::with_capacity(num_tokens);
        let mut phoneme_strs: Vec<String> = Vec::with_capacity(num_tokens);

        // If a word is in the dictionary, then it must have phonemes.
        // The phonemes will always be longer than the word, often significantly.
        //
        // Thus, the widths for known words are always computed from the phonemes.
        // (And, until there is alignment with the raw strings, they are used in all cases.)
        let mut widths: Vec<usize> = Vec::with_capacity(num_tokens);
        for i in 0..num_tokens {
            match self.get_entry(i) {
                Some(e) => {
                    dict_keys.push(e.dict_key());
                    phoneme_strs.push(format!("{}", e.phonemes));
                }
                None => {
                    let token_text = self.get_text(i);
                    dict_keys.push(format!("<span class=\"missing\">{}</span>", token_text));
                    phoneme_strs.push(format!("{: ^1$}", "?", token_text.len()));
                    // Centers the ?.
                }
            }
            widths.push(phoneme_strs.last().unwrap().len());
        }

        // Start the line by shifting over by the line number prefix (assumed "NN. ").
        out.push_str("     . ");
        for i in 0..num_tokens {
            // "Make the minimum field width the value of the '1'st argument (widths[i]), by
            // left-justifying the string ('<'), and filling the rest with '.'".
            //
            // Note that using "1$" in the format specifier has weird effects on the positional
            // arguments for the rest of the specifier, so it is best to put these all at the end.
            out.push_str(&format!("{:.<1$}  ", dict_keys[i], widths[i]));
        }

        // Again with the shift, and the previous EOL this time too.
        out.push_str("\n     . ");
        for i in 0..num_tokens {
            out.push_str(&format!("{}  ", phoneme_strs[i]));
        }
        out.push('\n');
    }
}

impl ToHtml for dictionary::Entry {
    fn append_html_to(&self, out: &mut String) {
        out.push_str(&format!(
            "(<span class=\"phonemes\">{}</span>); \
             <span class=\"entry_aux\">variant={}, \
             syllables={}</span>",
            &self.phonemes,
            &self.variant,
            self.num_syllables()
        ));
    }
}

/// Handler for the root (/) page.
#[rocket::get("/")]
fn index() -> Template {
    let context = HashMap::<&str, &str>::new();
    return Template::render("index", context);
}

/// Starts the Rocket HTTP server and awaits until the server shuts down.
///
/// Args:
///
/// * `shelf` - An already-initialized collection of dictionaries.
pub async fn run(shelf: dictionary::Shelf) {
    println!("*****************************************************************");
    println!("*                                                               *");
    println!("*  Launching Web Server.                                        *");
    println!("*                                                               *");
    println!("*  Type Control-C in the Terminal to stop the server.           *");
    println!("*                                                               *");
    println!("*****************************************************************");

    let result = rocket::build()
        .manage(ServerState {
            shelf: Mutex::new(shelf),
        })
        .attach(Template::fairing())
        .mount("/", routes![index, lookup, analyze, api_lookup])
        .mount("/static", rocket::fs::FileServer::from("static/"))
        .launch()
        .await;
    if let Err(e) = result {
        println!("***** Failed to launch web server. *****");
        // Drop the error to get a Rocket-formatted panic.
        drop(e);
    };
}
