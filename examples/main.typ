// #import "@preview/citegeist:0.2.0": load-bibliography
// #import "@local/citegeist:0.2.1": load-bibliography
#import "@local/citegeist:0.2.2": load-bibliography

#let bibtex_string = read("custom.bib")
#let bib = load-bibliography(bibtex_string, keep-raw-names: false)

#bib.bender-koller-2020-climbing

