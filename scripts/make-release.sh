#! /bin/bash

set -euo pipefail

PACKAGE=citegeist
VERSION=${1:-}
DOC_PATHS=(docs)

if [ -z "$VERSION" ];
then
    echo "You need to specify a version number."
    exit 1
fi


RELEASE_DIR="release/preview/$PACKAGE/$VERSION"
PLUGIN_DIR="plugin/$PACKAGE/plugin"

# Check that WASM is up to date.

pushd "$PLUGIN_DIR"
cargo build --target wasm32-unknown-unknown --release
popd


# Put together release

rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

cp lib.typ "$RELEASE_DIR/lib.typ"
cp README.md "$RELEASE_DIR/"
cp LICENSE "$RELEASE_DIR/"
cp "$PLUGIN_DIR/target/wasm32-unknown-unknown/release/$PACKAGE.wasm" "$RELEASE_DIR/"

EXCLUDES=()
for path in "${DOC_PATHS[@]}"; do
    if [ -d "$path" ]; then
        mkdir -p "$RELEASE_DIR/$path"
        cp -R "$path"/. "$RELEASE_DIR/$path"/
        EXCLUDES+=("/$path/**")
    elif [ -f "$path" ]; then
        mkdir -p "$RELEASE_DIR/$(dirname "$path")"
        cp "$path" "$RELEASE_DIR/$path"
        EXCLUDES+=("/$path")
    fi
done

for path in "${DOC_PATHS[@]}"; do
    if [ -e "$RELEASE_DIR/$path" ]; then
        # In the submitted package README, links to repository files should
        # point to the local copies above. Files in `exclude` remain available
        # on Universe.
        DOC_PATH="$path" perl -0pi -e '
            my $path = quotemeta($ENV{"DOC_PATH"});
            s{https://github\.com/alexanderkoller/typst-citegeist/(?:blob|tree)/main/($path(?:[/?#][^)\s]*)?)}{$1}g;
            s{https://raw\.githubusercontent\.com/alexanderkoller/typst-citegeist/(?:refs/heads/)?main/($path(?:[/?#][^)\s]*)?)}{$1}g;
        ' "$RELEASE_DIR/README.md"
    fi
done

if grep -Eq 'https://github\.com/alexanderkoller/typst-citegeist/(blob|tree)/main/|https://raw\.githubusercontent\.com/alexanderkoller/typst-citegeist/(refs/heads/)?main/' "$RELEASE_DIR/README.md"; then
    echo "README.md still contains links to the main branch of this repository." >&2
    echo "Link to a local release file instead, and add that file to DOC_PATHS if needed." >&2
    exit 1
fi

# replace version in typst.toml
sed "s/VERSION/$VERSION/g" typst-template.toml > "$RELEASE_DIR/typst.toml"
if [ ${#EXCLUDES[@]} -gt 0 ]; then
    {
        echo
        echo "exclude = ["
        for exclude in "${EXCLUDES[@]}"; do
            echo "  \"$exclude\","
        done
        echo "]"
    } >> "$RELEASE_DIR/typst.toml"
fi

echo "Package is ready for release in $RELEASE_DIR."
