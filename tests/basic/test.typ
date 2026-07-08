
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


#let extended-name-bib = "
@book{extended,
  author = {given=Simon, prefix=de, family=Beumont, useprefix=true and given=Jean Pierre Simon, given-i=JPS, prefix=de la, prefix-i=d, family=Rousse, id=rousse-jps},
  title = {Extended Names},
  year = {2026},
}
"

#let bib = load-bibliography(extended-name-bib)
#let authors = bib.extended.parsed_names.author

#assert.eq(authors.at(0).family, "Beumont")
#assert.eq(authors.at(0).prefix, "de")
#assert.eq(authors.at(0).at("use-prefix"), true)
#assert(not authors.at(0).keys().contains("given-initials"))
#assert(not authors.at(0).keys().contains("prefix-initials"))
#assert(not authors.at(0).keys().contains("id"))

#assert.eq(authors.at(1).family, "Rousse")
#assert.eq(authors.at(1).given, "Jean Pierre Simon")
#assert.eq(authors.at(1).prefix, "de la")
#assert.eq(authors.at(1).at("given-initials"), "JPS")
#assert.eq(authors.at(1).at("prefix-initials"), "d")
#assert.eq(authors.at(1).id, "rousse-jps")
#assert(not authors.at(1).keys().contains("use-prefix"))
