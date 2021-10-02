# Poet

"poet" is a helper.

It can analyze prose to determine (or guess) whether the prose is a
sonnet or haiku, including checking for rhyming and form.  It also has
a basic rhyming dictionary.

poet includes a web server, or it can take input from a text file and output
the analysis in a terminal.

## Usage

To run the poet web server:

```sh
$ cd <the directory where poet is>
$ ./poet -s
```
then go to http://127.0.0.1:8000/ in a web browser.

You can quit the server with Control-C in the terminal.

To run poet in the terminal on a file input:

```sh
$ cd <the directory where poet is>
$ ./poet -i <PATH, e.g. examples/stella-1.txt>
```

Generally, poet only knows about words in its dictionaries. These are loaded
at startup and read-only. However, there's a hacky form for fetching unknown
words. The output can be copied into userdict.dict so that it will be
included in the next run.

These files are included in a release package of `poet`:
  * `poet`: The binary.
  * `cmudict.dict`: The base dictionary. Required.
    This can also be downlaoded from https://github.com/cmusphinx/cmudict/raw/master/cmudict.dict
  * `userdict.dict`: Optional. Your own per-word additions in the cmudict format.
  * `examples/`: Some random snippets of poetry I use for testing.
  * `static/`: files for the web server.
  * `templates/`: files for the web server.

Feedback, feature requests, and bug reports welcome. If you want to
mess with the presentation (styles/formatting) of most of the things
on the web server, you can edit static & templates.

Also, if you get some weird "unidentified developer" errors, try running
this once:

```sh
$ xattr -l poet; xattr -d com.apple.quarantine poet
```

## Why poet?

I was recently nerd-sniped:

```
From: Someone who has written hundreds of sonnets
Date: Wed, Oct 7, 2020 at 7:08 AM
Subject: There's no quick way to create a rhyming dictionary is there?
To: Me

I'm frustrated having tapped out existing rhyming dictionaries' capacity (they
cluster on common words, limiting vocab) and they don't allow you to add new
words. I'm wondering if there'd be any more comprehensive search tool and
database if I understood UNIX or regular expressions or something. My next-best
alternative is to keep a word bank in a spreadsheet with columns separated by
final phoneme.
```

This reminded me of the `sphinx` phonetic dictionary that I'd run into in
undergrad, so I used that and shared a `grep`/`awk`/`sort` recipe to invert the
dictionary, for finding rhymes. It seemed like there was more potential,
though.

A little while later, I was looking for gift ideas, and I also wanted to
explore Rust with a significant project, so `poet` was born.

## Documentation

Technical notes and decisions are stored with the source in `src/`.

