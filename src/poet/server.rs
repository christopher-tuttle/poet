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
    let mut result = String::with_capacity(8192); // Arbitrary.

    let shelf = state.shelf.lock().unwrap();
    let dict = shelf.over_all();
    let stanzas = snippet::get_stanzas_from_text(&req.text, dict);
    for s in stanzas {
        result.push_str(&s.summarize_to_text());
        result.push('\n');
    }

    let mut context = HashMap::<&str, &str>::new();
    context.insert("raw_analysis", result.as_str());
    return Template::render("analyze", context);
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
