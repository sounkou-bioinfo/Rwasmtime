#!/usr/bin/env Rscript

rust_src <- file.path("src", "rust", "src")
cargo_toml <- file.path("src", "rust", "Cargo.toml")
if (!dir.exists(rust_src)) stop("missing Rust core source directory: ", rust_src, call. = FALSE)
if (!file.exists(cargo_toml)) stop("missing Rust Cargo.toml: ", cargo_toml, call. = FALSE)

rust_files <- list.files(rust_src, pattern = "[.]rs$", recursive = TRUE, full.names = TRUE)
if (!length(rust_files)) stop("no Rust core source files found", call. = FALSE)

read_one <- function(path) paste(readLines(path, warn = FALSE), collapse = "\n")

strip_rust_comments_and_strings <- function(x) {
  # This is a lightweight scanner for boundary checks, not a Rust parser. It
  # removes comments plus normal string/char literals before token matching.
  chars <- strsplit(x, "", fixed = TRUE)[[1L]]
  out <- character(length(chars))
  i <- 1L
  n <- length(chars)
  state <- "code"
  while (i <= n) {
    ch <- chars[[i]]
    nx <- if (i < n) chars[[i + 1L]] else ""
    if (state == "code") {
      if (ch == "/" && nx == "*") {
        out[[i]] <- " "; out[[i + 1L]] <- " "; i <- i + 2L; state <- "block_comment"; next
      }
      if (ch == "/" && nx == "/") {
        out[[i]] <- " "; out[[i + 1L]] <- " "; i <- i + 2L; state <- "line_comment"; next
      }
      if (ch == '"') {
        out[[i]] <- " "; i <- i + 1L; state <- "string"; next
      }
      if (ch == "'") {
        out[[i]] <- " "; i <- i + 1L; state <- "char"; next
      }
      out[[i]] <- ch; i <- i + 1L; next
    }
    if (state == "block_comment") {
      out[[i]] <- if (ch == "\n") "\n" else " "
      if (ch == "*" && nx == "/") { out[[i + 1L]] <- " "; i <- i + 2L; state <- "code" } else { i <- i + 1L }
      next
    }
    if (state == "line_comment") {
      out[[i]] <- if (ch == "\n") "\n" else " "
      if (ch == "\n") state <- "code"
      i <- i + 1L; next
    }
    if (state == "string") {
      out[[i]] <- if (ch == "\n") "\n" else " "
      if (ch == "\\") { if (i < n) out[[i + 1L]] <- " "; i <- i + 2L } else { if (ch == '"') state <- "code"; i <- i + 1L }
      next
    }
    if (state == "char") {
      out[[i]] <- if (ch == "\n") "\n" else " "
      if (ch == "\\") { if (i < n) out[[i + 1L]] <- " "; i <- i + 2L } else { if (ch == "'") state <- "code"; i <- i + 1L }
      next
    }
  }
  paste(out, collapse = "")
}

forbidden_rust_patterns <- c(
  "\\bSEXP\\b", "\\bSEXPREC\\b", "\\bALTREP\\b", "\\bPROTECT\\b", "\\bUNPROTECT\\b",
  "\\bR_xlen_t\\b", "\\bR_NilValue\\b", "\\bRf_[A-Za-z0-9_]+\\b", "\\bR_[A-Za-z0-9_]+\\b",
  "\\bsavvy(::|_)" , "\\bextendr(_api|::)?\\b", "\\blibR_sys\\b", "\\bRobj\\b",
  "\\bRinternals\\b", "\\bRdefines\\b", "\\bRembedded\\b"
)

violations <- character()
for (path in rust_files) {
  code <- strip_rust_comments_and_strings(read_one(path))
  for (pattern in forbidden_rust_patterns) {
    hits <- unique(unlist(regmatches(code, gregexpr(pattern, code, perl = TRUE))))
    if (length(hits)) {
      violations <- c(violations, sprintf("%s: %s", path, paste(hits, collapse = ", ")))
    }
  }
}
if (length(violations)) {
  stop(sprintf(
    "R-free Rust core boundary violation: src/rust/src must not use R/Savvy/SEXP API tokens\n%s",
    paste(unique(violations), collapse = "\n")
  ), call. = FALSE)
}

cargo <- readLines(cargo_toml, warn = FALSE)
# Feature names may mention future integration flags, but actual dependencies in
# the scaffold must not include R/Savvy crates.
in_deps <- FALSE
bad_deps <- character()
for (line in cargo) {
  trimmed <- trimws(sub("#.*$", "", line))
  if (!nzchar(trimmed)) next
  if (grepl("^\\[", trimmed)) {
    in_deps <- grepl("^\\[dependencies\\]", trimmed)
    next
  }
  if (in_deps && grepl("^(savvy|extendr-api|libR-sys|harp|Rinternals)\\b", trimmed)) {
    bad_deps <- c(bad_deps, trimmed)
  }
}
if (length(bad_deps)) {
  stop(sprintf(
    "R-free Rust core boundary violation: forbidden R/Savvy dependency in %s:\n%s",
    cargo_toml,
    paste(bad_deps, collapse = "\n")
  ), call. = FALSE)
}

cat(sprintf("R-free Rust core boundary check ok: %d source files\n", length(rust_files)))
