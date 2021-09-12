//! HTTP server components for poet.

use rocket::form::Form;
use rocket::serde::Serialize;
use rocket::State;
use rocket_dyn_templates::Template;
use std::collections::BTreeMap;
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

    if let Some(v) = state.dict.lookup(term) {
        if v.len() > 1 {
            // FIXME: Ignoring this case.
            println!(
                "Warning: {} has multiple results and only rendering the first.",
                term
            );
        }
        let entry = &v[0];
        context.entry_info = Some(format!("{:?}", entry));
        for word in state.dict.similar(term).words {
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
    if let Some(v) = state.dict.lookup(term) {
        if v.len() > 1 {
            // FIXME: Ignoring this case.
            println!(
                "Warning: {} has multiple results and only returning one.",
                term
            );
        }
        let entry = &v[0];
        let similar_info = state.dict.similar(term);
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

/// Stores the data about a single word in the input snippet, for passing to the template.
#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
struct SingleWordAnalysis<'a> {
    text: String,
    css_class: &'a str,
    syllables: i32,
}

/// Handler for a POST form to analyze a block of prose / snippet.
#[post("/analyze", data = "<req>")]
fn analyze(state: &State<ServerState>, req: Form<AnalyzeRequest>) -> Template {
    let mut context = rocket_dyn_templates::tera::Context::new();

    // For the moment, there are two different outputs here:
    // 1. The "raw" / terminal-like debug strings of all the word lookups, passed to the
    //    template to be inserted in a <pre> tag.
    // 2. A more colorful / styled version, showing the words with relevant data.
    //
    // Both are produced below and passed together to the template.
    //
    // TODO: Improve the second version to replace the first.
    // TODO: Refactor duplicated code below into a chart-like form.
    let stanzas = snippet::get_stanzas_from_text(&req.text, &state.dict);

    // Case 1, the terminal style.
    let mut raw_style_result = String::with_capacity(8192); // Arbitrary.
    for s in &stanzas {
        raw_style_result.push_str(&s.summarize_to_text());
        raw_style_result.push('\n');
    }
    context.insert("raw_analysis", raw_style_result.as_str());

    // Case 2, the colored spans.
    let mut annotations: Vec<Vec<SingleWordAnalysis>> = vec![];
    for s in &stanzas {
        for l in &s.lines {
            let mut line_annotations: Vec<SingleWordAnalysis> = vec![];
            for t in &l.tokens {
                let mut word_info = SingleWordAnalysis {
                    text: t.text.clone(),
                    css_class: "missing",
                    syllables: 0,
                };
                if let Some(entry) = &t.entry {
                    word_info.css_class = "found";
                    word_info.syllables = entry.syllables;
                }
                line_annotations.push(word_info);
            }
            annotations.push(line_annotations);
        }
    }

    context.insert("lines", &annotations);
    return Template::render("analyze", context.into_json());
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
