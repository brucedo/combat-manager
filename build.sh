#!/bin/env bash

RELEASE=false
CLEAN=false
BIN_SRC=
BIN_NAME="shadowrun"
CFG_NAME="Rocket.toml"
DIST=dist
STATIC="resources/static"
TEMPLATES="resources/templates"
BUILD_SUFFIX=""
DIST_STATIC="${DIST}"/${STATIC}
DIST_TEMPLATES="${DIST}"/${TEMPLATES}

while true; do
    case "$1" in
        -R | --release ) RELEASE=true; shift ;;
        -C | --clean ) CLEAN=true; shift ;;
        * ) break;
    esac
done

if [ $CLEAN == true ] ;
then
    cargo clean
    [ -d "${DIST}/" ] && rm -R "${DIST}/"
    exit 0;
fi


if [ $RELEASE == true ] ; 
then
    BIN_SRC="target/release"
    BUILD_SUFFIX="$BUILD_SUFFIX--release"
else
    BIN_SRC="target/debug"
fi

# Build Linux release...
cargo build "$BUILD_SUFFIX"

SRC_BIN_NAME="${BIN_SRC}/${BIN_NAME}"

[ ! -d "${DIST_STATIC}/" ] && mkdir -p "${DIST_STATIC}"
[ ! -d "${DIST_TEMPLATES}/" ] && mkdir -p "${DIST_TEMPLATES}"


cp "${SRC_BIN_NAME}" "${DIST}/"
cp "${CFG_NAME}" "${DIST}/"
cp -R ${STATIC}/* "${DIST_STATIC}/" 
cp -R ${TEMPLATES}/* "${DIST_TEMPLATES}/"

# package Linux release
tar -cvJf shadowrun-linux-x86_64.tar.xz ${DIST}/

# clear Linux bin in preparation for windows cross build
rm "${DIST}"/"${BIN_NAME}"

# build Windows release.  This should all be a bit less copy pasta from above...
cargo build --target x86_64-pc-windows-gnu "$BUILD_SUFFIX"

if [ $RELEASE == true ] ; 
then
    BIN_SRC="target/x86_64-pc-windows-gnu/release"
    BUILD_SUFFIX="$BUILD_SUFFIX--release"
else
    BIN_SRC="target/x86_64-pc-windows-gnu/debug"
fi

SRC_BIN_NAME="${BIN_SRC}/${BIN_NAME}.exe"
cp "${SRC_BIN_NAME}" "${DIST}/"

echo $SRC_BIN_NAME and $BIN_SRC

# package Windows release
zip -9r shadowrun-windows-x86_64.zip ${DIST}/