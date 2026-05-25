#import "/tests/test-lib.typ": *

#let bib = load-bibliography("
@online{protected-author,
  title = {Protected Author},
  author = {{Typst Team} and Doe, John and John {NASA} Smith},
  year = {2024},
}")

#let authors = bib.protected-author.parsed_names.author
#let metadata = bib.protected-author.name_metadata.author

#assert.eq(authors.len(), 3)
#assert.eq(metadata.len(), 3)
#assert.eq(authors.at(0).family, "Typst Team")
#assert.eq(authors.at(0).given, "")
#assert.eq(authors.at(0).keys().len(), 4)
#assert.eq(metadata.at(0).verbatim, true)
#assert.eq(metadata.at(0).literal, true)

#assert.eq(authors.at(1).family, "Doe")
#assert.eq(authors.at(1).given, "John")
#assert.eq(authors.at(1).keys().len(), 4)
#assert.eq(metadata.at(1).verbatim, false)
#assert.eq(metadata.at(1).literal, false)

#assert.eq(authors.at(2).family, "Smith")
#assert.eq(authors.at(2).given, "John NASA")
#assert.eq(authors.at(2).keys().len(), 4)
#assert.eq(metadata.at(2).verbatim, true)
#assert.eq(metadata.at(2).literal, false)
