#!/bin/env bash

RELEASE=false
CLEAN=false
BIN_SRC=
BIN_NAME="shadowrun"
CFG_NAME="Rocket.toml"
DIST=dist
STATIC="resources/static"
TEMPLATES="resources/templates"
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
    [ -d "${DIST}/" ] && rm -R "${DIST}/"
    exit 0;
fi

if [ $RELEASE == true ] ; 
then
    BIN_SRC="target/release"
else
    BIN_SRC="target/debug"
fi

BIN_NAME="${BIN_SRC}/${BIN_NAME}"

[ ! -d "${DIST_STATIC}/" ] && mkdir -p "${DIST_STATIC}"
[ ! -d "${DIST_TEMPLATES}/" ] && mkdir -p "${DIST_TEMPLATES}"

cp "${BIN_NAME}" "${DIST}/"
cp "${CFG_NAME}" "${DIST}/"
cp -R "${STATIC}" "${DIST_STATIC}" 
cp -R "${TEMPLATES}" "${DIST_TEMPLATES}"
