#!/usr/bin/env Rscript

if (!requireNamespace("Rtinycc", quietly = TRUE)) {
  stop("Rtinycc is required for the C API Rust backend exercise", call. = FALSE)
}
if (!requireNamespace("Rwasmtime", quietly = TRUE)) {
  stop("Rwasmtime must be installed before running the C API Rust backend exercise", call. = FALSE)
}

header <- system.file("include", "rwasmtime.h", package = "Rwasmtime")
lib <- file.path(system.file("libs", package = "Rwasmtime"), paste0("Rwasmtime", .Platform$dynlib.ext))
if (!nzchar(header) || !file.exists(header)) stop("installed rwasmtime.h not found", call. = FALSE)
if (!file.exists(lib)) stop("installed Rwasmtime native library not found", call. = FALSE)

allowed_symbols <- c(
  "rwasmtime_c_api_version",
  "rwasmtime_status_name",
  "rwasmtime_error_status",
  "rwasmtime_error_message",
  "rwasmtime_error_release",
  "rwasmtime_runtime_build",
  "rwasmtime_runtime_call_core",
  "rwasmtime_runtime_release"
)
nm <- Sys.which("nm")
if (!nzchar(nm)) stop("nm is required for the installed C API symbol audit", call. = FALSE)
nm_args <- if (.Platform$OS.type == "unix") c("-D", "--defined-only", lib) else c("-g", lib)
nm_out <- system2(nm, nm_args, stdout = TRUE, stderr = TRUE)
nm_status <- attr(nm_out, "status")
if (!is.null(nm_status) && nm_status != 0L) {
  writeLines(nm_out)
  stop("nm failed during installed C API symbol audit", call. = FALSE)
}
exported_names <- sub("^.*[[:space:]]", "", nm_out)
exported_rwasmtime <- sort(unique(grep("^rwasmtime_", exported_names, value = TRUE)))
extra <- setdiff(exported_rwasmtime, allowed_symbols)
missing <- setdiff(allowed_symbols, exported_rwasmtime)
if (length(extra) || length(missing)) {
  stop(sprintf(
    "installed C API symbol audit failed\nextra:\n%s\nmissing:\n%s",
    paste(sprintf("  - %s", extra), collapse = "\n"),
    paste(sprintf("  - %s", missing), collapse = "\n")
  ), call. = FALSE)
}
cat(sprintf("installed C API symbol audit ok: %d rwasmtime_* symbols\n", length(allowed_symbols)))

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

int rwasmtime_c_api_rust_backend_roundtrip(void) {
  rwasmtime_runtime_options_t opts = {0};
  static const uint8_t add_wasm[85] = {
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0c, 0x02, 0x60,
    0x02, 0x7f, 0x7f, 0x01, 0x7f, 0x60, 0x01, 0x7f, 0x01, 0x7f, 0x03, 0x03,
    0x02, 0x00, 0x01, 0x07, 0x0e, 0x02, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00,
    0x04, 0x73, 0x70, 0x69, 0x6e, 0x00, 0x01, 0x0a, 0x28, 0x02, 0x07, 0x00,
    0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b, 0x1e, 0x01, 0x01, 0x7f, 0x20, 0x00,
    0x21, 0x01, 0x02, 0x40, 0x03, 0x40, 0x20, 0x01, 0x45, 0x0d, 0x01, 0x20,
    0x01, 0x41, 0x01, 0x6b, 0x21, 0x01, 0x0c, 0x00, 0x0b, 0x0b, 0x20, 0x00,
    0x0b
  };
  rwasmtime_runtime_t *runtime = 0;
  rwasmtime_error_t *err = 0;
  rwasmtime_status_t status;
  rwasmtime_core_value_t args[2] = {{0}};
  rwasmtime_core_value_t results[1] = {{0}};
  rwasmtime_core_call_options_t call_opts = {0};
  size_t results_len = 999;
  const char *message;

  opts.struct_size = sizeof opts;
  opts.compiler_strategy = "cranelift";
  opts.opt_level = "speed";
  opts.parallel = RWASMTIME_TOGGLE_TRUE;
  opts.component_model = RWASMTIME_TOGGLE_FALSE;
  opts.simd = RWASMTIME_TOGGLE_TRUE;
  opts.relaxed_simd = RWASMTIME_TOGGLE_FALSE;
  opts.relaxed_simd_deterministic = RWASMTIME_TOGGLE_FALSE;

  status = rwasmtime_runtime_build(&opts, &runtime, &err);
  if (status != RWASMTIME_OK) return 10 + (int)status;
  if (runtime == 0) return 20;
  if (err != 0) return 21;

  call_opts.struct_size = sizeof call_opts;
  call_opts.has_fuel = 1;
  call_opts.fuel = 1000;
  args[0].tag = RWASMTIME_CORE_VALUE_I32;
  args[0].i64_value = 8;
  args[1].tag = RWASMTIME_CORE_VALUE_I32;
  args[1].i64_value = 34;
  status = rwasmtime_runtime_call_core(runtime, add_wasm, sizeof add_wasm, "add", args, 2, &call_opts, results, 1, &results_len, &err);
  if (status != RWASMTIME_OK) return 22 + (int)status;
  if (err != 0) return 26;
  if (results_len != 1) return 27;
  if (results[0].tag != RWASMTIME_CORE_VALUE_I32 || results[0].i64_value != 42) return 28;

  results_len = 999;
  status = rwasmtime_runtime_call_core(runtime, add_wasm, sizeof add_wasm, "add", args, 2, &call_opts, 0, 0, &results_len, &err);
  if (status != RWASMTIME_INVALID_ARGUMENT) return 50 + (int)status;
  if (results_len != 0) return 60;
  if (rwasmtime_error_status(err) != RWASMTIME_INVALID_ARGUMENT) return 61;
  message = rwasmtime_error_message(err);
  if (message == 0 || strstr(message, "result capacity") == 0) return 62;
  rwasmtime_error_release(err);
  err = 0;

  rwasmtime_runtime_release(runtime);
  runtime = 0;

  opts.relaxed_simd_deterministic = RWASMTIME_TOGGLE_TRUE;
  status = rwasmtime_runtime_build(&opts, &runtime, &err);
  if (status != RWASMTIME_INVALID_ARGUMENT) return 30 + (int)status;
  if (runtime != 0) return 40;
  if (rwasmtime_error_status(err) != RWASMTIME_INVALID_ARGUMENT) return 41;
  message = rwasmtime_error_message(err);
  if (message == 0 || strstr(message, "relaxed_simd_deterministic") == 0) return 42;
  rwasmtime_error_release(err);
  return 0;
}
'

status <- Rtinycc::tcc_compile_string(state, code)
if (!identical(as.integer(status), 0L)) {
  stop(sprintf("Rtinycc C API Rust backend compile failed with status %s", status), call. = FALSE)
}
status <- Rtinycc::tcc_relocate(state)
if (!identical(as.integer(status), 0L)) {
  stop(sprintf("Rtinycc C API Rust backend relocation failed with status %s", status), call. = FALSE)
}
result <- Rtinycc::tcc_call_symbol(state, "rwasmtime_c_api_rust_backend_roundtrip", return = "int")
if (!identical(as.integer(result), 0L)) {
  stop(sprintf("C API Rust backend probe failed with code %s", result), call. = FALSE)
}

cat(sprintf("Rtinycc C API Rust backend check ok: %s\n", header))
