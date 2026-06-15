#!/usr/bin/env Rscript

allowed <- c(
  "rwasmtime_c_api_version",
  "rwasmtime_status_name",
  "rwasmtime_error_status",
  "rwasmtime_error_message",
  "rwasmtime_error_release",
  "rwasmtime_runtime_build",
  "rwasmtime_runtime_call_core",
  "rwasmtime_runtime_release"
)

args <- commandArgs(trailingOnly = TRUE)
header <- if (length(args) >= 1L) args[[1L]] else file.path("inst", "include", "rwasmtime.h")
source <- if (length(args) >= 2L) args[[2L]] else file.path("src", "c_api.c")
header <- normalizePath(header, winslash = "/", mustWork = TRUE)
source <- normalizePath(source, winslash = "/", mustWork = TRUE)

read_one <- function(path) paste(readLines(path, warn = FALSE), collapse = "\n")

strip_c_comments_and_strings <- function(x) {
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

extract_declared <- function(text) {
  m <- gregexpr("RWASMTIME_API[[:space:]][^;]+?\\b(rwasmtime_[A-Za-z0-9_]+)[[:space:]]*\\(", text, perl = TRUE)
  hits <- regmatches(text, m)[[1L]]
  if (!length(hits) || identical(hits, "")) return(character())
  unique(sub("(?s)^.*\\b(rwasmtime_[A-Za-z0-9_]+)[[:space:]]*\\(.*$", "\\1", hits, perl = TRUE))
}

extract_public_defs <- function(text) {
  lines <- unlist(strsplit(text, "\n", fixed = TRUE))
  # Public definitions in the current tiny C API have one of these return
  # forms. This deliberately ignores static helpers and call sites such as
  # `return rwasmtime_set_error(...)`.
  pattern <- paste0(
    "^[[:space:]]*(uint32_t|const[[:space:]]+char[[:space:]]*\\*|",
    "rwasmtime_status_t|void)[[:space:]]*",
    "(rwasmtime_[A-Za-z0-9_]+)[[:space:]]*\\("
  )
  lines <- grep(pattern, lines, value = TRUE, perl = TRUE)
  if (!length(lines)) return(character())
  unique(sub(paste0(pattern, ".*$"), "\\2", lines, perl = TRUE))
}

header_symbols <- sort(extract_declared(strip_c_comments_and_strings(read_one(header))))
source_symbols <- sort(extract_public_defs(strip_c_comments_and_strings(read_one(source))))
allowed <- sort(allowed)

fail <- function(label, values) {
  if (length(values)) {
    stop(sprintf("%s:\n%s", label, paste(sprintf("  - %s", sort(values)), collapse = "\n")), call. = FALSE)
  }
}

fail("C API header declares unreviewed symbols", setdiff(header_symbols, allowed))
fail("C API source defines unreviewed public symbols", setdiff(source_symbols, allowed))
fail("C API header is missing allowed symbols", setdiff(allowed, header_symbols))
fail("C API source is missing allowed public symbols", setdiff(allowed, source_symbols))
fail("C API header/source symbol mismatch", union(setdiff(header_symbols, source_symbols), setdiff(source_symbols, header_symbols)))

cat(sprintf("C API symbol whitelist check ok: %d symbols\n", length(allowed)))
