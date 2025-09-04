#import "@preview/citegeist:0.2.0": load-bibliography
// #import "@local/citegeist:0.1.1": load-bibliography

#let bibtex_string = read("custom.bib")
#let bib = load-bibliography(bibtex_string)

#bib.bender-koller-2020-climbing

