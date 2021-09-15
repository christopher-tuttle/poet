//! HTTP server components for poet.

use rocket::form::Form;
use rocket::serde::Serialize;
use rocket::State;
use rocket_dyn_templates::Template;
use std::collections::HashMap;

use crate::poet::*;

/// A container for data owned by web server that's available for all requests.
///
/// XXX: This is assumed to be thread safe but it is only incidentally so right now. Fix it.
struct ServerState {
    dict: dictionary::Dictionary,
}

/// A Context for populating the lookup template.
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct LookupTemplateContext<'a> {
    query: &'a str,
    entry_info: Option<String>,
    similar_words: Vec<String>,
}

/// Handler for querying the dictionary for a single term.
#[get("/lookup?<term>")]
fn lookup(state: &State<ServerState>, term: &str) -> Template {
    let mut context = LookupTemplateContext {
        query: term,
        entry_info: None,
        similar_words: vec![],
    };

    if let Some(entry) = state.dict.lookup(term) {
        context.entry_info = Some(format!("{:?}", entry));
        for word in state.dict.similar(term) {
            context.similar_words.push(word);
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
    if let Some(entry) = state.dict.lookup(term) {
        let mut similar = state.dict.similar(term);
        let num_synonyms = similar.len();
        if similar.len() > 8 {
            similar.truncate(9);
            similar[8] = String::from("...");
        }
        let result: String = format!(
            "{} (<code>{}</code>) [{} syllables] with {} synonyms like: {}",
            term,
            entry.phonemes.join(" "),
            entry.syllables,
            num_synonyms,
            similar.join(", ")
        );
        return result;
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
    let snippets = snippet::get_snippets_from_text(&req.text, &state.dict);
    for s in snippets {
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
pub async fn run(dictionary: dictionary::Dictionary) {
    println!("*****************************************************************");
    println!("*                                                               *");
    println!("*  Launching Web Server.                                        *");
    println!("*                                                               *");
    println!("*  Type Control-C in the Terminal to stop the server.           *");
    println!("*                                                               *");
    println!("*****************************************************************");

    let result = rocket::build()
        .manage(ServerState { dict: dictionary })
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
