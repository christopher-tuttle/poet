# Poet Technical Notes and Decisions

_To start, this is just a scratch pile of things that may be useful._

## Feature thoughts

  * Run analysis from command-line on a text file. Also useful for reg tests?
  * Evolution of the analysis:
    * Load the dictionary.
    * Analyze input text one token at a time, ignoring structure.
    * Analyze per-line structure.
    * Analyze full text against the syllable pattern.
    * Stretch: Compare similarity of words for rhyming.
  * Evolution of a web-based interface for data:
    * Print summary statistics and a sample of the dictionary.
    * Look up words in the dictionary.
    * Identify and show words that may rhyme with an input.
    * AJAX-ify it.
  * Evolution of a web-based interface for poems:
    * Render output of text analysis as a static page.
    * Very basic web form that analyzes a provided poem.
    * Per-word annotations of syllables, emphasis, etc.
  * Infrastructure learning extras
    * Authentication / login -- probably oauth.
    * Debug pages, e.g. `statusz` and `varz`.
    * Live heap and cpu profiles.
    * Debugging info in the request responses.
  * Big Stretch
    * Modifying the dictionary.
    * Supporting phonetic annotation.
    * Integration with Google Docs to pull text from there.

## Webserver crate: Rocket.rs

I looked at `warp`, `hyper`, and `rocket`, and it looks like `rocket` is
well-used, developer-friendly, and actively staffed. So Rocket it is.

## Notes on word analysis

_There's no quick way to create a rhyming dictionary is there?_

```
From: Chris Tuttle <>
Date: Wed, Oct 7, 2020 at 2:10 PM
Subject: Re: There's no quick way to create a rhyming dictionary is there?
```

So if you have some table of words with their last phoneme separated out, I can
help you with the shell/regex/whatnot tricks to make it easier to search
through. I'm assuming that you don't at the moment. In which case:

I don't know of an easy way, but I don't think it's impossible. If I were
trying to do this, I'd start by looking at the latest text-to-speech research
and tools, since this problem can be solved by having two tables, one mapping
words to phonemes and the other stringing together phonemes into sound. (At
least this was the way it used to be solved; I don't know how much of it is
one-shot with neural networks now.)

CMU's Sphinx is one of the notable ones -- it's been around since well before I was in college. Here are some pages:
   * The raw english dictionary: https://raw.githubusercontent.com/cmusphinx/cmudict/master/cmudict.dict
   * The enclosing repo/project: https://github.com/cmusphinx/cmudict
   * A description of the dictionary format: http://www.speech.cs.cmu.edu/cgi-bin/cmudict
   * The sphinx tool for editing/messing with the dictionary (provides more context): https://cmusphinx.github.io/wiki/tutorialdict/

Going to the command line, I fire up Terminal and ....
```sh
$ mkdir Desktop/sphinx-text
$ cd Desktop/sphinx-text/
$ curl -o cmudict.dict https://raw.githubusercontent.com/cmusphinx/cmudict/master/cmudict.dict
$ wc -l cmudict.dict
  135154 cmudict.dict            # <-- number of lines in the file
$ tail cmudict.dict              # <-- tail means look at the last n (default 10) lines
zylstra Z IH1 L S T R AH0
zyman Z AY1 M AH0 N
zynda Z IH1 N D AH0
zysk Z AY1 S K
zyskowski Z IH0 S K AO1 F S K IY0
zyuganov Z Y UW1 G AA0 N AA0 V
zyuganov(2) Z UW1 G AA0 N AA0 V
zyuganov's Z Y UW1 G AA0 N AA0 V Z
zyuganov's(2) Z UW1 G AA0 N AA0 V Z
zywicki Z IH0 W IH1 K IY0
```

Let's see what rhymes with zyskowski.
```sh
$ egrep 'K AO[0-9]* F S K IY[0-9]*$' cmudict.dict
bakowski B AH0 K AO1 F S K IY0
bankowski B AH0 NG K AO1 F S K IY0
bartkowski B ER0 T K AO1 F S K IY0
...
zuchowski(2) Z UW0 K AO1 F S K IY0
zukowski Z AH0 K AO1 F S K IY0
zyskowski Z IH0 S K AO1 F S K IY0
```

A breakdown of that grep:
```sh
$ egrep 'K AO[0-9]* F S K IY[0-9]*$' cmudict.dict
```

`egrep` is the same as `grep -E`, where the -E means "extended", which gives some
useful regex shortcuts like being able to write `[0-9]` to mean "all the
characters from 0 through 9 inclusive" instead of having to spell out
`[0123456789]`.

`K AO1 F S K IY0` is the end of the phonetic line for the kowski part of zyskowski.

In that encoding, the digits indicate which syllables get the stress, but
that's overly restrictive for just the endings. SO I replaced the `1` and `0`
digits with `[0-9]*`, which means any number (0 or more occurrences of), of the
ascii characters between 0-9, inclusive. And then finally, I also said the match
had to be at the end of the line. If you were to remove the `$`, you'd see some
possessives pop in there, which have the extra `Z` sound.

A couple more examples. Finding similar words for **rouge**:
```sh
$ egrep rouge cmudict.dict
baton-rouge B AE1 T AH0 N R UW1 JH
baton-rouge's B AE1 T AH0 N R UW1 JH IH0 Z
rouge R UW1 ZH
rougeau R UW0 ZH OW1
$ egrep 'UW[0-9]* ZH$' cmudict.dict
bruges(2) B R UW1 ZH
rouge R UW1 ZH
```

And **bayous**:
```sh
$ egrep bayous cmudict.dict
bayous B AY1 UW0 Z
$ egrep 'UW[0-9]* Z$' cmudict.dict
... 293 lines ...
```

So! That can get you started on a brute-forcy way. But we can also maybe make
this a little better...

Here's how to create a dictionary that puts similar words kinda next to each other in the file:
  1. Strip out the comments from a couple lines in the file (see $ egrep '#' cmudict.dict to see what I mean).
  1. (Optional) remove all digits from every line, so you don't need to do the [0-9] garbage when searching.
  1. Reverse all the tokens on each line. (I stole this snippet from stackoverflow.)
  1. Sort (now by reversed phonetics)
  1. Write to a file.

```sh
$ sed 's/ *#.*$//g' cmudict.dict | tr -d '0123456789' | awk '{for(i=NF;i>=1;i--) printf "%s ", $i;print ""}' | sort > reversed.dict
```

And since the sort order will be affected by removing the emphasis, you can
also do this, which sorts before stripping the digits.

```sh
$ sed 's/ *#.*$//g' cmudict.dict | awk '{for(i=NF;i>=1;i--) printf "%s ", $i;print ""}' | sort | tr -d '0123456789' > reversed-keep-emphasis.dict
```

TBH I'm not sure how much it matters. Here's a bit of the diff (reversed on
left, reversed-with-emphasis on right), and I see a few lines jumping around a
little.

**TODO: Insert diff**

So dunno about that.

For these files, I expect that you'd mostly be browsing. If you have a text
editor that's having a hard time with the size, using `less` in the terminal
works ok.

```sh
$ less reversed-keep-emphasis.dict
```

From here:
  * `q` to quit
  * `/` followed by text to search, e.g. `/flower<return>`. then use `n` and `N` to jump between matches
  * `h` for help

Either way, hopefully this is enough to get you going.

