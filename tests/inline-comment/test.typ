
#import "/tests/test-lib.typ": *

#let basic-bib = "
@book{knuth1990,
  author = {Knuth, Donald E.},
  maintitle = {Maintitle},
  mainsubtitle = {Sub},
  volume = {1},
  part = {2}, % here's an inline comment
  year = 1990,
  publisher = {Addison-Wesley Professional},
  title  = {The {\TeX} Book},
}
"

#let bib = load-bibliography(basic-bib)
