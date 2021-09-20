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

    // For the moment, there are many different outputs here:
    // 1. The "raw" / terminal-like debug strings of all the word lookups, passed to the
    //    template to be inserted in a <pre> tag.
    // 2. A more colorful / styled version, showing the words with relevant data.
    // 3. A rust-generated, colored version, which needs to happen because I'm on a deadline.
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
        for i in s.interpretations().take(1) {
            // XXX
            raw_style_result.push_str(&format!("{}\n", &i));
            match snippet::is_shakespearean_sonnet(&i) {
                Ok(_) => {
                    raw_style_result.push_str("This is a valid Shakespearean Sonnet!\n");
                }
                Err(v) => {
                    raw_style_result.push_str("Errors and warnings:\n");
                    for e in &v {
                        raw_style_result.push_str(&format!("{}\n", e));
                    }
                }
            }
        }
        raw_style_result.push_str("\n\n");
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
                    // XXX WRONG -- DOESN't SUPPORT MULTIPLE ENTRIES
                    word_info.syllables = entry[0].num_syllables();
                }
                line_annotations.push(word_info);
            }
            annotations.push(line_annotations);
        }
    }
    context.insert("lines", &annotations);

    // *** HACK ALERT *** //
    //
    let mut html = String::with_capacity(32768); // Arbitrary.

    for s in &stanzas {
        // BLOCK 1: THE INTERPRETATION.
        // XXX: FOR NOW JUST USING THE FIRST INTERPRETATION.
        for i in s.interpretations().take(1) {
            // XXX
            html.push_str("<pre>");
            html.push_str(&format!("{}\n", stanza_view_to_html(&i))); // FIXME TO MAKE NEW RENDERING THINGY
            match snippet::is_shakespearean_sonnet(&i) {
                Ok(_) => {
                    html.push_str("This is a valid Shakespearean Sonnet!\n");
                }
                Err(v) => {
                    html.push_str("<span class=\"error_header\">Errors and warnings:</span>\n");
                    for e in &v {
                        use snippet::ClassifyError::*;
                        match &e {
                            StanzaError(s) => html.push_str(&format!(
                                "<span class=\"stanza_warning\">{}</span>\n",
                                &e
                            )),
                            LineError(l, s) => html
                                .push_str(&format!("<span class=\"line_warning\">{}</span>\n", &e)),
                        }
                    }
                }
            }
            html.push_str("\n");
            html.push_str("</pre>");
        }
        // BLOCK 2: THE STANZA WORDS.
        html.push_str("<pre>");
        html.push_str(&summarize_stanza_to_html(&s)); // hack hack hack
        html.push_str("\n");
        html.push_str("</pre>");
    }

    context.insert("prose_html", &html);
    return Template::render("analyze", context.into_json());
}

fn stanza_view_to_html(stanza: &snippet::StanzaView) -> String {
    let mut out = String::with_capacity(8192); // Arbitrary.
    for line in &stanza.lines {
        out.push_str(&line_view_to_html(&line));
        out.push('\n');
    }
    return out;
}

fn line_view_to_html(line: &snippet::LineView) -> String {
    let mut out = String::with_capacity(1024); // Arbitrary.

    out.push_str(&format!(
        "{:02} {:2}. {}\n",
        line.line.num,
        line.line.index + 1,
        &line.line.raw_text
    ));
    // Start with just blasting everything there, and then make it pretty / evenly spaced.
    let num_tokens = line.indices.len();

    let mut dict_keys: Vec<String> = Vec::with_capacity(num_tokens);
    let mut phoneme_strs: Vec<String> = Vec::with_capacity(num_tokens);

    // If a word is in the dictionary, then it must have phonemes.
    // The phonemes will always be longer than the word, often significantly.
    //
    // Thus, the widths for known words are always computed from the phonemes.
    // (And, until there is alignment with the raw strings, they are used in all cases.)
    let mut widths: Vec<usize> = Vec::with_capacity(num_tokens);
    for i in 0..num_tokens {
        match line.get_entry(i) {
            Some(e) => {
                dict_keys.push(e.dict_key());
                phoneme_strs.push(format!("{}", e.phonemes));
            }
            None => {
                let token_text = line.get_text(i);
                dict_keys.push(format!("<span class=\"missing\">{}</span>", token_text));
                phoneme_strs.push(format!("{: ^1$}", "?", token_text.len())); // Centers the ?.
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
    return out;
}

fn summarize_entry_to_html(entry: &dictionary::Entry) -> String {
    format!("(<span class=\"phonemes\">{}</span>); <span class=\"entry_aux\">variant={}, syllables={}</span>",
        &entry.phonemes, &entry.variant, entry.num_syllables())
}

// XXX SO HACKY
fn summarize_stanza_to_html(stanza: &snippet::Stanza) -> String {
    let mut out = String::with_capacity(8192); // Arbitrary.

    if let Some(title) = &stanza.title {
        out.push_str(&format!("TITLE: {}\n", &title));
    }

    for line in &stanza.lines {
        out.push_str(&line.raw_text);
        out.push('\n');
        for token in &line.tokens {
            if let Some(entries) = &token.entry {
                for (i, entry) in entries.iter().enumerate() {
                    if i == 0 {
                        out.push_str(&format!(
                            "\t{}: {}\n",
                            &token.text,
                            summarize_entry_to_html(&entry)
                        ));
                    } else {
                        out.push_str(&format!(
                            "\t{}: {}\n",
                            &" ".repeat(token.text.len()),
                            summarize_entry_to_html(&entry)
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
    if stanza.has_unknown_words() {
        out.push_str(&format!(
            "Warning: The text has some unknown words. Analysis may suffer.\n"
        ));
    }
    return out;
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
