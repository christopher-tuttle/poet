//! HTTP server components for poet.

use rocket::response::content;
use rocket::form::Form;
use rocket::State;

use crate::poet::*;

/// A container for data owned by web server that's available for all requests.
///
/// XXX: This is assumed to be thread safe but it is only incidentally so right now. Fix it.
struct ServerState {
    dict: dictionary::Dictionary
}

/// Handler for querying the dictionary for a single term.
#[get("/lookup?<term>")]
fn lookup(state: &State<ServerState>, term: &str) -> content::Html<String> {
    // TODO: Add validation to the query term.
    // TODO: Improve the fidelity and separation of the rhyming words, and their presentation.
    let mut result: String = format!("<p>The query was <b>{}</b>.</p>", term);
    if let Some(entry) = state.dict.lookup(term) {
        result.push_str(format!("<p>Found: <code>{:?}</code></p>", entry).as_str());
        result.push_str(format!("<p>Potential rhymes: ").as_str());
        for word in state.dict.similar(term) {
            result.push_str(format!("{}, ", word).as_str());
        }
        result.push_str("</p>");

    } else {
        result.push_str("<p><b><font color=red>Word not found</font></b></p>");
    }
    return content::Html(result);
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
fn analyze(state: &State<ServerState>, req: Form<AnalyzeRequest>) -> content::Html<String> {
    // TODO: Migrate this to templates.
    // TODO: Refactor duplicated code below into a chart-like form.
    let mut result: String = format!("<p><b>Analysis of text:</b></p>");
    result.push_str("<pre>\n");
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
    return content::Html(result);
}

/// Handler for the root (/) page.
#[rocket::get("/")]
fn index() -> content::Html<String> {
    // TODO: Migrate this to templates.
    content::Html(format!(r#"
        <h1>/ˈpōət/</h1>
        <p><i>Look up a single word:</i>
        <form action="/lookup">
          <input name="term" type=text>
          <input type="submit" value="Lookup">
        </form>
        </p>
        <p><i>Analyze some text:</i>
        <form action="/analyze" method="post">
          <textarea name="text" rows=30 cols=100></textarea>
          <input type="submit" value="Go">
        </form>
        </p>
        "#))
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
        .mount("/", routes![index, lookup, analyze])
        .launch().await;
    if let Err(e) = result {
        println!("***** Failed to launch web server. *****");
        // Drop the error to get a Rocket-formatted panic.
        drop(e);
    };
}
