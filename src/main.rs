#[macro_use]
extern crate lazy_static;
extern crate clap;
use clap::{App, Arg};

mod poet;

use crate::poet::*;

fn handle_term_query(query: &str, dict: &dyn dictionary::Dictionary) {
    if let Some(entry) = dict.lookup(query) {
        println!("Found {:?}", entry);
        for word in dict.similar(query).words {
            println!("\tsimilar word: {:?}", word);
        }
    } else {
        println!("Not found: {}", query);
    }
}

#[macro_use]
extern crate rocket;

#[rocket::main]
async fn main() {
    let matches = App::new("poet")
        .version("0.1.0")
        .arg(
            Arg::with_name("dict")
                .short("d")
                .long("dict")
                .value_name("FILE")
                .help("Path to the cmudict.dict dictionary file.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("userdict")
                .short("u")
                .long("userdict")
                .value_name("FILE")
                .help("Path to the user dictionary, in cmudict format.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("query")
                .short("q")
                .long("query")
                .value_name("WORD")
                .help("Looks up WORD in the dictionary and returns info on it.")
                .takes_value(true)
                .conflicts_with_all(&["input", "server"]),
        )
        .arg(
            Arg::with_name("input")
                .short("i")
                .long("input")
                .value_name("FILE")
                .help("Analyzes the lines in the given text file.")
                .takes_value(true)
                .conflicts_with("server"),
        )
        .arg(
            Arg::with_name("server")
                .short("s")
                .long("server")
                .help("Launches the poet web server."),
        )
        .get_matches();

    let cmudict_path = matches.value_of("dict").unwrap_or("./cmudict.dict");
    let userdict_path = matches.value_of("userdict").unwrap_or("./userdict.dict");

    let mut shelf = poet::dictionary::Shelf::new();
    shelf
        .init_cmudict(cmudict_path)
        .expect("Failed to read cmudict file!");
    if let Err(e) = shelf.init_userdict(userdict_path) {
        println!(
            "Failed to read userdict file. Skipping and continuing. Error={}",
            e
        );
    }

    if let Some(q) = matches.value_of("query") {
        // TODO: Exit with a failure status value if lookup failed.
        handle_term_query(q, shelf.over_all());
        return;
    }

    if let Some(path) = matches.value_of("input") {
        // TODO: Handle errors more gracefully.
        snippet::analyze_one_file_to_terminal(path, shelf.over_all());
        return;
    }

    if matches.is_present("server") {
        server::run(shelf).await;
        return;
    }

    println!(
        "\nUSAGE (see poet --help for more):\n\n\
             Either run poet as a server ('poet --server') or provide it with a file or query,\n\
             such as 'poet --input examples/shakespeare-all.txt'.\n"
    );
}
