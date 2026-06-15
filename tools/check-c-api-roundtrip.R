#!/usr/bin/env Rscript

if (!requireNamespace("Rtinycc", quietly = TRUE)) {
  stop("Rtinycc is required for the C API roundtrip exercise", call. = FALSE)
}
if (!requireNamespace("Rwasmtime", quietly = TRUE)) {
  stop("Rwasmtime must be installed before running the C API roundtrip exercise", call. = FALSE)
}

library(Rwasmtime)

header <- system.file("include", "rwasmtime.h", package = "Rwasmtime")
lib <- file.path(system.file("libs", package = "Rwasmtime"), paste0("Rwasmtime", .Platform$dynlib.ext))
if (!nzchar(header) || !file.exists(header)) stop("installed rwasmtime.h not found", call. = FALSE)
if (!file.exists(lib)) stop("installed Rwasmtime native library not found", call. = FALSE)

state <- Rtinycc::tcc_state(output = "memory")
status <- Rtinycc::tcc_add_include_path(state, dirname(header))
if (!identical(as.integer(status), 0L)) {
  stop(sprintf("Rtinycc could not add include path %s", dirname(header)), call. = FALSE)
}
status <- Rtinycc::tcc_add_file(state, lib)
if (!identical(as.integer(status), 0L)) {
  stop(sprintf("Rtinycc could not add installed native library %s", lib), call. = FALSE)
}

code <- '
#include "rwasmtime.h"
#include <string.h>

int rwasmtime_c_api_roundtrip(void) {
  rwasmtime_runtime_options_t opts = {0};
  rwasmtime_runtime_t *runtime = 0;
  rwasmtime_error_t *err = 0;
  rwasmtime_status_t status;
  rwasmtime_core_value_t call_results[1] = {{0}};
  size_t call_results_len = 999;
  const char *message;

  if (rwasmtime_c_api_version() != RWASMTIME_C_API_VERSION) return 10;
  if (strcmp(rwasmtime_status_name(RWASMTIME_NOT_IMPLEMENTED), "not_implemented") != 0) return 11;
  if (strcmp(rwasmtime_status_name((rwasmtime_status_t)999), "unknown") != 0) return 12;

  status = rwasmtime_runtime_call_core(0, (const uint8_t *)"x", 1, "run", 0, 0, 0, call_results, 1, &call_results_len, &err);
  if (status != RWASMTIME_INVALID_ARGUMENT) return 13 + (int)status;
  if (call_results_len != 0) return 14;
  if (rwasmtime_error_status(err) != RWASMTIME_INVALID_ARGUMENT) return 15;
  rwasmtime_error_release(err);
  err = 0;

  opts.struct_size = sizeof opts;
  opts.compiler_strategy = "auto";
  opts.opt_level = "speed";
  opts.parallel = RWASMTIME_TOGGLE_TRUE;
  opts.component_model = RWASMTIME_TOGGLE_TRUE;
  opts.simd = RWASMTIME_TOGGLE_TRUE;
  opts.relaxed_simd = RWASMTIME_TOGGLE_FALSE;
  opts.relaxed_simd_deterministic = RWASMTIME_TOGGLE_TRUE;

  status = rwasmtime_runtime_build(&opts, &runtime, &err);
  if (status != RWASMTIME_NOT_IMPLEMENTED) return 20 + (int)status;
  if (runtime != 0) return 30;
  if (rwasmtime_error_status(err) != RWASMTIME_NOT_IMPLEMENTED) return 31;
  message = rwasmtime_error_message(err);
  if (message == 0 || strstr(message, "not implemented") == 0) return 32;
  rwasmtime_error_release(err);
  err = 0;

  status = rwasmtime_runtime_build(&opts, 0, &err);
  if (status != RWASMTIME_INVALID_ARGUMENT) return 40 + (int)status;
  if (rwasmtime_error_status(err) != RWASMTIME_INVALID_ARGUMENT) return 50;
  rwasmtime_error_release(err);
  rwasmtime_runtime_release(runtime);
  return 0;
}
'

status <- Rtinycc::tcc_compile_string(state, code)
if (!identical(as.integer(status), 0L)) {
  stop(sprintf("Rtinycc C API roundtrip compile failed with status %s", status), call. = FALSE)
}
status <- Rtinycc::tcc_relocate(state)
if (!identical(as.integer(status), 0L)) {
  stop(sprintf("Rtinycc C API roundtrip relocation failed with status %s", status), call. = FALSE)
}
result <- Rtinycc::tcc_call_symbol(state, "rwasmtime_c_api_roundtrip", return = "int")
if (!identical(as.integer(result), 0L)) {
  stop(sprintf("C API roundtrip probe failed with code %s", result), call. = FALSE)
}

cat(sprintf("Rtinycc C API roundtrip check ok: %s\n", header))
