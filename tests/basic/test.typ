
#import "/tests/test-lib.typ": *

// #import "@local/citegeist:0.2.1": *

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

#let bib = load-bibliography(basic-bib)
