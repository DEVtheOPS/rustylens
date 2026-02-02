#!/bin/bash
# Build script for gh-pages
# Copies site/ contents and update.json to dist/

set -e

rm -rf dist
mkdir -p dist

cp -r site/* dist/
cp update.json dist/
cp .nojekyll dist/

echo "Built to dist/"
