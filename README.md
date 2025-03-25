# citegeist: Direct bibtex access for Typst

This package reads a Bibtex file and returns its contents as as a [Typst dictionary](https://typst.app/docs/reference/foundations/dictionary/). It does not attempt to typeset a bibliography and is not interested in CSL styles; all it does is to return the raw Bibtex entries. It leaves all further processing to your Typst code.

Citegeist is a thin wrapper around the 
