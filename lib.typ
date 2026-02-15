

#let load-bibliography(bibtex-string, keep-raw-names: true) = {
  let p = plugin("citegeist.wasm")
  let options = if keep-raw-names { bytes((1,)) } else { bytes((0,)) }
  let serialized = p.get_bib_map(bytes(bibtex-string), options)
  let bib-map = cbor(serialized)

  bib-map
}
