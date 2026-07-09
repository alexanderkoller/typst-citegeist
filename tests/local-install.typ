#import "@local/citegeist:0.3.1": load-bibliography

#let bibtex = "
@article{first,
  author = {Doe, Jane and Smith, John},
  title = {A {LOCAL} Install Test},
  journal = {Journal of Smoke Tests},
  year = {2026},
}

@book{first,
  author = {Duplicate, Later},
  title = {Should Be Ignored},
  year = {2027},
}

@book{second,
  author = {given=Jean Pierre Simon, given-i=JPS, prefix=de la, prefix-i=d, family=Rousse, id=rousse-jps},
  title = {Extended Names},
  year = {2025},
}
"

#let bib = load-bibliography(
  bibtex,
  on-duplicate: "keep-first",
  sentence-case-titles: true,
  source: "local-install-test.bib",
)

#assert.eq(bib.keys(), ("first", "second"))
#assert.eq(bib.first.position, 0)
#assert.eq(bib.second.position, 1)
#assert.eq(bib.first.entry_type, "article")
#assert.eq(bib.first.fields.title, "A LOCAL install test")
#assert.eq(bib.first.fields.author, "Doe, Jane and Smith, John")

#let first-authors = bib.first.parsed_names.author
#assert.eq(first-authors.len(), 2)
#assert.eq(first-authors.at(0).family, "Doe")
#assert.eq(first-authors.at(0).given, "Jane")
#assert.eq(first-authors.at(1).family, "Smith")
#assert.eq(first-authors.at(1).given, "John")

#let second-author = bib.second.parsed_names.author.at(0)
#assert.eq(second-author.family, "Rousse")
#assert.eq(second-author.given, "Jean Pierre Simon")
#assert.eq(second-author.prefix, "de la")
#assert.eq(second-author.at("given-initials"), "JPS")
#assert.eq(second-author.at("prefix-initials"), "d")
#assert.eq(second-author.id, "rousse-jps")

Local Citegeist test passed.
