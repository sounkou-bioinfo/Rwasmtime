#!/bin/sh
set -eu
for path in DESCRIPTION NAMESPACE README.Rmd AGENTS.md Makefile R/api.R tests/tinytest.R inst/tinytest inst/include/rwasmtime.h src/rust/Cargo.toml; do
  test -e "$path" || { echo "missing $path" >&2; exit 1; }
done
printf '%s\n' "layout ok"
