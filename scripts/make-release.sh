#! /bin/bash

set -euo pipefail

PACKAGE=citegeist
UPDATE_SOURCES=false
VERSION=""
declare -a DOC_PATHS=()
PLUGIN_DIR="plugin/$PACKAGE/plugin"

usage() {
    echo "Usage: $0 [--update-sources] VERSION"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --update-sources)
            UPDATE_SOURCES=true
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
        *)
            if [ -n "$VERSION" ]; then
                echo "Unexpected argument: $1" >&2
                usage >&2
                exit 1
            fi
            VERSION=$1
            ;;
    esac
    shift
done

if [ -z "$VERSION" ];
then
    echo "You need to specify a version number." >&2
    usage >&2
    exit 1
fi

update_sources() {
    local version=$1

    VERSION="$version" PACKAGE="$PACKAGE" perl -0pi -e '
        my $version = $ENV{"VERSION"};
        my $package = quotemeta($ENV{"PACKAGE"});
        s{\@(preview|local)/$package:\d+\.\d+\.\d+}{\@$1/$ENV{"PACKAGE"}:$version}g;
        s{\A(.*?^## Changelog\s+?)## (?:Unreleased|\d+\.\d+\.\d+)}{$1## $version}ms;
    ' README.md examples/main.typ tests/local-install.typ

    VERSION="$version" perl -0pi -e '
        my $version = $ENV{"VERSION"};
        s{^version = "\d+\.\d+\.\d+"}{version = "$version"}m;
    ' "$PLUGIN_DIR/Cargo.toml"

    if VERSION="$version" PACKAGE="$PACKAGE" perl -ne '
        BEGIN {
            $version = $ENV{"VERSION"};
            $package = $ENV{"PACKAGE"};
            $found_stale = 0;
        }
        if (m{\@(preview|local)/\Q$package\E:(\d+\.\d+\.\d+)} && $2 ne $version) {
            print STDERR "$ARGV:$.: found stale \@$1/$package:$2 import\n";
            $found_stale = 1;
        }
        END {
            exit($found_stale ? 0 : 1);
        }
    ' README.md examples/main.typ tests/local-install.typ; then
        echo "Source update failed: found a stale @preview/$PACKAGE or @local/$PACKAGE import." >&2
        exit 1
    fi

    if ! grep -q "^## $version$" README.md; then
        echo "Source update failed: README.md changelog was not updated to $version." >&2
        exit 1
    fi

    if ! grep -q "^version = \"$version\"$" "$PLUGIN_DIR/Cargo.toml"; then
        echo "Source update failed: $PLUGIN_DIR/Cargo.toml was not updated to $version." >&2
        exit 1
    fi
}

check_package_versions() {
    local version=$1

    if VERSION="$version" PACKAGE="$PACKAGE" perl -ne '
        BEGIN {
            $version = $ENV{"VERSION"};
            $package = $ENV{"PACKAGE"};
            $found_stale = 0;
        }
        if (m{\@(preview|local)/\Q$package\E:(\d+\.\d+\.\d+)} && $2 ne $version) {
            print STDERR "$ARGV:$.: found stale \@$1/$package:$2 import\n";
            $found_stale = 1;
        }
        END {
            exit($found_stale ? 0 : 1);
        }
    ' README.md examples/main.typ tests/local-install.typ "$RELEASE_DIR/README.md"; then
        echo "Release check failed: found a stale @preview/$PACKAGE or @local/$PACKAGE import." >&2
        exit 1
    fi

    if ! grep -q "^version = \"$version\"$" "$RELEASE_DIR/typst.toml"; then
        echo "Release check failed: $RELEASE_DIR/typst.toml was not updated to $version." >&2
        exit 1
    fi
}

RELEASE_DIR="release/preview/$PACKAGE/$VERSION"

if [ "$UPDATE_SOURCES" = true ]; then
    update_sources "$VERSION"
fi

# Check that WASM is up to date.

cargo build-wasm


# Put together release

rm -rf "$RELEASE_DIR"
mkdir -p "$RELEASE_DIR"

cp lib.typ "$RELEASE_DIR/lib.typ"
cp README.md "$RELEASE_DIR/"
cp LICENSE "$RELEASE_DIR/"
cp "$PLUGIN_DIR/target/wasm32-unknown-unknown/release/$PACKAGE.wasm" "$RELEASE_DIR/"

EXCLUDES=()
if [ "${#DOC_PATHS[@]}" -gt 0 ]; then
    for path in "${DOC_PATHS[@]}"; do
        if [ -d "$path" ]; then
            mkdir -p "$RELEASE_DIR/$path"
            cp -R "$path"/. "$RELEASE_DIR/$path"/
            EXCLUDES+=("/$path/**")
        elif [ -f "$path" ]; then
            mkdir -p "$RELEASE_DIR/$(dirname "$path")"
            cp "$path" "$RELEASE_DIR/$path"
            EXCLUDES+=("/$path")
        else
            echo "Configured documentation path does not exist: $path" >&2
            exit 1
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
fi

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

check_package_versions "$VERSION"

echo "Package is ready for release in $RELEASE_DIR."
