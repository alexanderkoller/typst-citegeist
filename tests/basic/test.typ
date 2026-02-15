
#import "/tests/test-lib.typ": *

#let basic-bib = "
@book{knuth1990,
  author = {Knuth, Donald E.},
  maintitle = {Maintitle},
  mainsubtitle = {Sub},
  volume = {1},
  part = {2},
  year = 1990,
  publisher = {Addison-Wesley Professional},
  title  = {The {\TeX} Book},
}


@misc{generalized-2025,
  author = {Katharina Stein and Nils Hodel and Michael Katz and Jörg Hoffmann and Alexander Koller},
  title = {Improved Generalized Planning with LLMs through Strategy Refinement and Reflection},
  year = {2025},
  howpublished = {Submitted to AAAI}
}

"


Hello World

#let bib = load-bibliography(basic-bib, sentence-case-titles: false)

#assert("knuth1990" in bib)
#assert.eq(bib.knuth1990.entry_type, "book")
#assert.eq(bib.knuth1990.fields.title, "The TeX Book")



#let bib = load-bibliography(basic-bib, sentence-case-titles: true)

#assert("knuth1990" in bib)
#assert.eq(bib.knuth1990.entry_type, "book")
#assert.eq(bib.knuth1990.fields.title, "The TeX book")
