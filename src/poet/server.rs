//! HTTP server components for poet.

use rocket::form::Form;
use rocket::State;
use rocket_dyn_templates::Template;
use rocket::serde::Serialize;
use std::collections::HashMap;

use crate::poet::*;

/// A container for data owned by web server that's available for all requests.
///
/// XXX: This is assumed to be thread safe but it is only incidentally so right now. Fix it.
struct ServerState {
    dict: dictionary::Dictionary
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
#[post("/analyze", data="<req>")]
fn analyze(state: &State<ServerState>, req: Form<AnalyzeRequest>) -> Template {
    // TODO: Migrate this to templates.
    // TODO: Refactor duplicated code below into a chart-like form.
    let mut result: String = format!("<p><b>Analysis of text:</b></p>");
    result.push_str("<pre>\n");

    let mut result = String::new();
    for line in req.text.lines() {
        if line.is_empty() {
            continue;
        }
        result.push_str(format!("{}\n", line).as_str());
        let mut num_syllables: i32 = 0;
        for token in line.trim().split_whitespace() {
            let key = snippet::normalize_for_lookup(token);
            if let Some(entry) = state.dict.lookup(&key) {
                result.push_str(format!("\t{}: {:?}\n", token, entry).as_str());
                num_syllables += entry.syllables;
            } else {
                result.push_str(format!("\t{}: None\n", token).as_str());
            }
        }
        result.push_str(format!("\t==> Line summary: {} syllables.\n", num_syllables).as_str());
        result.push_str("\n");
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
        .mount("/", routes![index, lookup, analyze])
        .mount("/static", rocket::fs::FileServer::from("static/"))
        .launch().await;
    if let Err(e) = result {
        println!("***** Failed to launch web server. *****");
        // Drop the error to get a Rocket-formatted panic.
        drop(e);
    };
}
