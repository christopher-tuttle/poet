"poet" is a helper.

It can analyze prose to determine (or guess) whether the prose is a
sonnet or haiku, including checking for rhyming and form.  It also has
a basic rhyming dictionary.

poet includes a web server, though it can take input from a text file
and output the analysis in a terminal.

To run the poet web server:

  $ cd <the directory where poet is>
  $ ./poet -s
  then go to http://127.0.0.1:8000/ in a web browser.

  You can quit the server with Control-C in the terminal.

To run poet in the terminal on a file input:

  $ cd <the directory where poet is>
  $ ./poet -i <PATH, e.g. examples/stella-1.txt>

Generally, poet only knows about words in its dictionaries. These are loaded
at startup and read-only. However, there's a hacky form for fetching unknown
words. The output can be copied into userdict.dict so that it will be
included in the next run.

Explanation of files in this directory:
  - poet: The binary.

  - cmudict.dict: The base dictionary. Required.
        from https://github.com/cmusphinx/cmudict/raw/master/cmudict.dict
  - userdict.dict: Optional. Your own per-word additions in the cmudict format.

  - examples/ : Some random snippets of poetry I use for testing.

  - static/: files for the web server.
  - templates/: files for the web server.

Feedback, feature requests, and bug reports welcome. If you want to
mess with the presentation (styles/formatting) of most of the things
on the web server, you can edit static & templates.


Also, if you get some weird "unidentified developer" errors, try running
this once:

$ xattr -l poet; xattr -c poet

