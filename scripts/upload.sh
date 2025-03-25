#! /bin/bash

PACKAGE=citegeist
VERSION=$1

if [ -z "$VERSION" ];
then
    echo "You need to specify a version number."
    exit 1
fi

GITHUB_PACKAGE=typst-packages-$PACKAGE-$VERSION
SRC_DIR=preview/$PACKAGE/$VERSION
TGT_DIR=$GITHUB_PACKAGE/packages/preview/$PACKAGE/$VERSION

cd release

# make sure that we have a clone of our fork
if [ ! -d typst-packages ]; then
    git clone https://github.com/alexanderkoller/$GITHUB_PACKAGE
fi

pushd $GITHUB_PACKAGE
git pull # synchronize with our own changes
git pull https://github.com/typst/packages # synchronize with global changes
popd

# make room
rm -rf $TGT_DIR
mkdir -p $TGT_DIR

# copy
cp -r $SRC_DIR/* $TGT_DIR/

# commit
pushd $GITHUB_PACKAGE
git add packages/preview/$PACKAGE/$VERSION/*
git commit -am "$PACKAGE:$VERSION"
popd

