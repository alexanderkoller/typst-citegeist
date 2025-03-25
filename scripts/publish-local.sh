#! /bin/bash

PACKAGE=citegeist

LOCAL="/Users/koller/Library/ApplicationSupport/typst/packages/local"
echo "$LOCAL"
mkdir -p "$LOCAL"

cp -r release/preview/$PACKAGE "$LOCAL"
