#!/usr/bin/env Rscript

if (!requireNamespace("Rtinycc", quietly = TRUE)) {
  stop("Rtinycc is required for the C API header exercise", call. = FALSE)
}

args <- commandArgs(trailingOnly = TRUE)
header <- if (length(args) >= 1L) args[[1L]] else file.path("inst", "include", "rwasmtime.h")
header <- normalizePath(header, winslash = "/", mustWork = TRUE)
include_dir <- dirname(header)

state <- Rtinycc::tcc_state(output = "memory")
status <- Rtinycc::tcc_add_include_path(state, include_dir)
if (!identical(as.integer(status), 0L)) {
  stop(sprintf("Rtinycc could not add include path %s", include_dir), call. = FALSE)
}

code <- '
#include "rwasmtime.h"

int rwasmtime_header_probe(void) {
  rwasmtime_runtime_options_t opts = {0};
  rwasmtime_core_call_options_t call_opts = {0};
  rwasmtime_core_value_t args[2] = {{0}};
  rwasmtime_core_value_t results[1] = {{0}};
  rwasmtime_runtime_t *runtime = 0;
  rwasmtime_error_t *err = 0;
  opts.struct_size = sizeof opts;
  opts.compiler_strategy = "auto";
  opts.opt_level = "speed";
  opts.parallel = RWASMTIME_TOGGLE_TRUE;
  opts.component_model = RWASMTIME_TOGGLE_TRUE;
  opts.simd = RWASMTIME_TOGGLE_TRUE;
  opts.relaxed_simd = RWASMTIME_TOGGLE_FALSE;
  opts.relaxed_simd_deterministic = RWASMTIME_TOGGLE_TRUE;
  call_opts.struct_size = sizeof call_opts;
  call_opts.has_fuel = 1;
  call_opts.fuel = 1000;
  args[0].tag = RWASMTIME_CORE_VALUE_I32;
  args[0].i64_value = 1;
  args[1].tag = RWASMTIME_CORE_VALUE_I32;
  args[1].i64_value = 2;
  results[0].tag = RWASMTIME_CORE_VALUE_I32;
  (void)runtime;
  (void)err;
  return (int)(RWASMTIME_C_API_VERSION + opts.struct_size + call_opts.struct_size + args[0].tag + results[0].tag + RWASMTIME_NOT_IMPLEMENTED);
}
'

status <- Rtinycc::tcc_compile_string(state, code)
if (!identical(as.integer(status), 0L)) {
  stop(sprintf("Rtinycc header compile failed with status %s", status), call. = FALSE)
}
status <- Rtinycc::tcc_relocate(state)
if (!identical(as.integer(status), 0L)) {
  stop(sprintf("Rtinycc header relocation failed with status %s", status), call. = FALSE)
}
probe <- Rtinycc::tcc_call_symbol(state, "rwasmtime_header_probe", return = "int")
if (!is.numeric(probe) || probe <= 0) stop("unexpected C API header probe result", call. = FALSE)

cat(sprintf("Rtinycc C API header compile check ok: %s\n", header))
