#!/usr/bin/env Rscript

args <- commandArgs(trailingOnly = TRUE)
paths <- if (length(args)) args else c(
  file.path("inst", "include", "rwasmtime.h"),
  file.path("src", "c_api.c")
)
paths <- normalizePath(paths, winslash = "/", mustWork = TRUE)

read_one <- function(path) {
  paste(readLines(path, warn = FALSE), collapse = "\n")
}

strip_c_comments_and_strings <- function(x) {
  # Small scanner, not a C parser. Good enough for boundary checks: remove
  # comments and string/character literals before looking for forbidden R API
  # tokens, so prose like "does not expose SEXP" is allowed in comments.
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
        out[[i]] <- " "
        out[[i + 1L]] <- " "
        i <- i + 2L
        state <- "block_comment"
        next
      }
      if (ch == "/" && nx == "/") {
        out[[i]] <- " "
        out[[i + 1L]] <- " "
        i <- i + 2L
        state <- "line_comment"
        next
      }
      if (ch == '"') {
        out[[i]] <- " "
        i <- i + 1L
        state <- "string"
        next
      }
      if (ch == "'") {
        out[[i]] <- " "
        i <- i + 1L
        state <- "char"
        next
      }
      out[[i]] <- ch
      i <- i + 1L
      next
    }
    if (state == "block_comment") {
      out[[i]] <- if (ch == "\n") "\n" else " "
      if (ch == "*" && nx == "/") {
        out[[i + 1L]] <- " "
        i <- i + 2L
        state <- "code"
      } else {
        i <- i + 1L
      }
      next
    }
    if (state == "line_comment") {
      out[[i]] <- if (ch == "\n") "\n" else " "
      if (ch == "\n") state <- "code"
      i <- i + 1L
      next
    }
    if (state == "string") {
      out[[i]] <- if (ch == "\n") "\n" else " "
      if (ch == "\\") {
        if (i < n) out[[i + 1L]] <- " "
        i <- i + 2L
      } else {
        if (ch == '"') state <- "code"
        i <- i + 1L
      }
      next
    }
    if (state == "char") {
      out[[i]] <- if (ch == "\n") "\n" else " "
      if (ch == "\\") {
        if (i < n) out[[i + 1L]] <- " "
        i <- i + 2L
      } else {
        if (ch == "'") state <- "code"
        i <- i + 1L
      }
      next
    }
  }
  paste(out, collapse = "")
}

for (path in paths) {
  text <- read_one(path)
  code <- strip_c_comments_and_strings(text)

  include_lines <- unlist(regmatches(code, gregexpr("^[[:space:]]*#[[:space:]]*include[^\n]*", code, perl = TRUE)))
  forbidden_includes <- grep("<(R|R_ext)/|<Rinternals\\.h>|<Rdefines\\.h>|<Rembedded\\.h>|<R_ext/", include_lines, value = TRUE, perl = TRUE)
  if (length(forbidden_includes)) {
    stop(sprintf(
      "R-free C API boundary violation in %s: forbidden R include(s):\n%s",
      path,
      paste(forbidden_includes, collapse = "\n")
    ), call. = FALSE)
  }

  forbidden_tokens <- c(
    "SEXP", "SEXPREC", "PROTECT", "UNPROTECT", "PROTECT_WITH_INDEX",
    "REPROTECT", "Rf_[A-Za-z0-9_]+", "R_[A-Za-z0-9_]+",
    "allocVector", "mkString", "ScalarInteger", "ScalarReal",
    "R_RegisterCCallable", "R_GetCCallable"
  )
  pattern <- paste0("\\b(", paste(forbidden_tokens, collapse = "|"), ")\\b")
  hits <- unique(unlist(regmatches(code, gregexpr(pattern, code, perl = TRUE))))
  # RWASMTIME_* macros are allowed; they are package C API names, not R API.
  hits <- hits[!grepl("^RWASMTIME", hits)]
  if (length(hits)) {
    stop(sprintf(
      "R-free C API boundary violation in %s: forbidden R API token(s): %s",
      path,
      paste(hits, collapse = ", ")
    ), call. = FALSE)
  }
}

cat(sprintf("R-free C API boundary check ok: %s\n", paste(paths, collapse = ", ")))
