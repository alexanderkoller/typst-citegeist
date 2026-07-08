#import "@preview/citegeist:0.3.0": load-bibliography

#let bibtex_string = read("custom.bib")
#let bib = load-bibliography(bibtex_string, keep-raw-names: false)

#bib.bender-koller-2020-climbing

