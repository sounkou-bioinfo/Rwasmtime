#!/usr/bin/env Rscript

fail <- function(...) stop(paste0(...), call. = FALSE)

read_text <- function(path) paste(readLines(path, warn = FALSE), collapse = "\n")

extract_savvy_symbols <- function(text) {
  matches <- gregexpr("SEXP[[:space:]]+(savvy_[A-Za-z0-9_]+__ffi)[[:space:]]*\\(", text, perl = TRUE)
  raw <- regmatches(text, matches)[[1L]]
  if (!length(raw) || identical(raw, character(0))) return(character())
  unique(sub("^SEXP[[:space:]]+(savvy_[A-Za-z0-9_]+__ffi)[[:space:]]*\\($", "\\1", raw, perl = TRUE))
}

extract_savvy_prototypes <- function(text) {
  matches <- gregexpr("SEXP[[:space:]]+savvy_[A-Za-z0-9_]+__ffi[[:space:]]*\\([^;{]*[;{]", text, perl = TRUE)
  raw <- regmatches(text, matches)[[1L]]
  if (!length(raw) || identical(raw, character(0))) return(setNames(character(), character()))
  normalized <- trimws(gsub("[[:space:]]+", " ", sub("[;{][[:space:]]*$", "", raw, perl = TRUE)))
  normalized <- gsub("( ", "(", normalized, fixed = TRUE)
  names(normalized) <- sub("^SEXP[[:space:]]+(savvy_[A-Za-z0-9_]+__ffi)[[:space:]]*\\(.*$", "\\1", normalized, perl = TRUE)
  normalized
}

for (path in c("configure", "src/Makevars.wasm.in", "src/wasm_stubs.c", "src/native_stubs.c", "src/rust/api.h")) {
  if (!file.exists(path)) fail("missing required wasm gate file: ", path)
}

wasm_makevars <- readLines("src/Makevars.wasm.in", warn = FALSE)
object_line <- grep("^OBJECTS[[:space:]]*=", wasm_makevars, value = TRUE)
if (!identical(length(object_line), 1L)) fail("src/Makevars.wasm.in must contain exactly one OBJECTS line")
if (!grepl("(^|[[:space:]])wasm_stubs[.]o($|[[:space:]])", object_line)) {
  fail("src/Makevars.wasm.in must link wasm_stubs.o")
}
forbidden <- c("native_stubs.o", "librwasmtime_savvy.a")
for (token in forbidden) {
  if (grepl(token, paste(wasm_makevars, collapse = "\n"), fixed = TRUE)) {
    fail("src/Makevars.wasm.in must not reference ", token)
  }
}

api_text <- read_text("src/rust/api.h")
api_symbols <- sort(extract_savvy_symbols(api_text))
api_prototypes <- extract_savvy_prototypes(api_text)
for (stub_path in c("src/wasm_stubs.c", "src/native_stubs.c")) {
  stub_text_for_proto <- read_text(stub_path)
  stub_symbols <- sort(extract_savvy_symbols(stub_text_for_proto))
  missing <- setdiff(api_symbols, stub_symbols)
  stale <- setdiff(stub_symbols, api_symbols)
  if (length(missing) || length(stale)) {
    fail(
      stub_path, " Savvy stub symbols are out of sync\nmissing stubs: ", paste(missing, collapse = ", "),
      "\nstale stubs: ", paste(stale, collapse = ", ")
    )
  }
  stub_prototypes <- extract_savvy_prototypes(stub_text_for_proto)
  drift <- api_symbols[api_prototypes[api_symbols] != stub_prototypes[api_symbols]]
  if (length(drift)) {
    fail(stub_path, " Savvy stub prototypes are out of sync: ", paste(drift, collapse = ", "))
  }
}
if (!length(api_symbols)) fail("no generated Savvy FFI symbols found in src/rust/api.h")

stub_text <- read_text("src/wasm_stubs.c")
if (!grepl("current webR/wasm build", stub_text, fixed = TRUE)) {
  fail("src/wasm_stubs.c must report the webR/wasm pending-backend boundary")
}
if (grepl("RWASMTIME_RUST_BACKEND=1", stub_text, fixed = TRUE)) {
  fail("src/wasm_stubs.c must not use the native reinstall error text")
}

makevars_path <- file.path("src", "Makevars")
had_makevars <- file.exists(makevars_path)
old_makevars <- if (had_makevars) readBin(makevars_path, what = "raw", n = file.info(makevars_path)$size) else raw()
on.exit({
  if (had_makevars) {
    writeBin(old_makevars, makevars_path, useBytes = TRUE)
  } else if (file.exists(makevars_path)) {
    unlink(makevars_path)
  }
}, add = TRUE)

status <- system2("./configure", "--host=wasm32-unknown-emscripten")
if (!identical(status, 0L)) fail("./configure --host=wasm32-unknown-emscripten failed")
configured <- readLines(makevars_path, warn = FALSE)
if (!identical(configured, wasm_makevars)) {
  fail("configure did not select src/Makevars.wasm.in for wasm32-unknown-emscripten")
}

status <- system2("./configure", "--host=wasm32-unknown-emscripten", env = c("RWASMTIME_RUST_BACKEND=1"))
if (!identical(status, 0L)) fail("RWASMTIME_RUST_BACKEND=1 ./configure --host=wasm32-unknown-emscripten failed")
configured <- readLines(makevars_path, warn = FALSE)
if (!identical(configured, wasm_makevars)) {
  fail("wasm configure must prefer src/Makevars.wasm.in over the native Rust backend")
}

cat(sprintf(
  "webR/wasm gate ok: configure selects wasm stubs and %d generated Savvy symbols are covered\n",
  length(api_symbols)
))
