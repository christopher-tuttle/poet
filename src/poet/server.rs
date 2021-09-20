//! HTTP server components for poet.

use rocket::form::Form;
use rocket::serde::Serialize;
use rocket::State;
use rocket_dyn_templates::Template;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::poet::*;

/// A container for data owned by web server that's available for all requests.
struct ServerState {
    dict: Mutex<dictionary::DictionaryImpl>,
}

/// A Context for populating the lookup template.
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct LookupTemplateContext<'a> {
    /// The word being looked up.
    query: &'a str,

    /// Metadata about the query word;
    entry_info: Option<String>,

    /// All of the words, sorted in decreasing order of similarity.
    similar_words: Vec<String>,

    /// All of the words, grouped in various ways (the key), sorted in decreasing order of
    /// simlarity.
    word_groups: BTreeMap<String, Vec<dictionary::SimilarWord>>,
}

/// Handler for querying the dictionary for a single term.
#[get("/lookup?<term>")]
fn lookup(state: &State<ServerState>, term: &str) -> Template {
    let mut context = LookupTemplateContext {
        query: term,
        entry_info: None,
        similar_words: vec![],
        word_groups: BTreeMap::new(),
    };

    let dict = state.dict.lock().unwrap();

    use dictionary::Dictionary;
    if let Some(v) = dict.lookup(term) {
        if v.len() > 1 {
            // FIXME: Ignoring this case.
            println!(
                "Warning: {} has multiple results and only rendering the first.",
                term
            );
        }
        let entry = &v[0];
        context.entry_info = Some(format!("{:?}", entry));
        for word in dict.similar(term).words {
            context.similar_words.push(word.word.clone());
            let group = format!("Rhymes with {} syllables", word.syllables);
            if let Some(v) = context.word_groups.get_mut(&group) {
                v.push(word.clone());
            } else {
                context.word_groups.insert(group, vec![word.clone()]);
            }
        }
    }
    return Template::render("lookup", context);
}

/// Handler for AJAX lookup of a term (`/api/lookup?term=<query>`).
///
/// The templating is all done server-side at the moment, so this endpoint returns HTML to
/// be inserted into the page.
#[get("/api/lookup?<term>")]
fn api_lookup(state: &State<ServerState>, term: &str) -> String {
    let dict = state.dict.lock().unwrap();

    use dictionary::Dictionary;
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

    let dict = state.dict.lock().unwrap();
    let stanzas = snippet::get_stanzas_from_text(&req.text, &*dict);
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
/// * `dictionary` - An already-initialized dictionary to use when handling all requests.
pub async fn run(dictionary: dictionary::DictionaryImpl) {
    println!("*****************************************************************");
    println!("*                                                               *");
    println!("*  Launching Web Server.                                        *");
    println!("*                                                               *");
    println!("*  Type Control-C in the Terminal to stop the server.           *");
    println!("*                                                               *");
    println!("*****************************************************************");

    let result = rocket::build()
        .manage(ServerState {
            dict: Mutex::new(dictionary),
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
