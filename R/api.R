#' Pipe-first Wasmtime embedding API
#'
#' Pipe-first R API for a Rust/Savvy-backed Wasmtime embedding. The default build
#' defines stable R-side constructors and honest pending-backend boundaries.
#' Native backend builds enable real core calls, WASIp1 commands, persistent core
#' sessions, host callbacks, typed linear-memory transport, and a core-memory
#' REPL protocol.
#'
#' @name Rwasmtime-api
#' @aliases
#'   wt_add_callback wt_aot_load wt_aot_save wt_app wt_arg_array wt_array_write
#'   wt_artifact_compatible wt_artifact_info wt_as_array wt_as_component wt_as_module wt_await
#'   wt_build_runtime wt_call wt_call_async wt_callback_policy wt_callbacks wt_cancel
#'   wt_compile wt_component wt_component_exports wt_component_imports wt_drain_callbacks wt_enable_features
#'   wt_exec wt_exports wt_free wt_imports wt_bindings wt_limit_callbacks wt_limit_fuel wt_limit_instances wt_limit_memory
#'   wt_limit_tables wt_limit_wall_time wt_limits wt_link_callbacks wt_link_wasi wt_linker
#'   wt_instantiate wt_memory wt_memory_grow wt_memory_read wt_memory_size wt_memory_view wt_memory_write
#'   wt_new_session wt_poll wt_prepare wt_repl wt_repl_close wt_repl_eval
#'   wt_repl_history wt_repl_info wt_repl_read wt_repl_send wt_result wt_runtime_spec
#'   wt_store wt_wasi wt_wasi_args wt_wasi_env wt_wasi_network wt_wasi_preopen
#'   wt_wasi_stdio wt_webr_repl wt_with_allocator wt_with_aot wt_with_arrays wt_with_callbacks
#'   wt_with_compiler wt_with_limits wt_with_runtime wt_with_temp_array wt_with_wasi wt_with_wit
#' @param .x Object to modify, build, query, or execute.
#' @param source Wasm module, component, or AOT artifact source.
#' @param kind Source kind.
#' @param strategy Compiler strategy.
#' @param opt_level Compiler optimization level.
#' @param parallel Whether compiler parallelism is enabled.
#' @param component_model,component_model_async,simd,relaxed_simd,relaxed_simd_deterministic,bulk_memory,multi_memory,memory64,threads,exceptions,legacy_exceptions,gc Optional runtime feature toggles. `NULL` leaves the existing value unchanged.
#' @param export Guest export name.
#' @param ... Additional arguments for the relevant verb.
#' @param .args Explicit list of guest arguments.
#' @param protocol REPL protocol.
#' @param eval_export Component export or logical evaluator name for a REPL guest.
#' @param prompt,continuation Prompt strings.
#' @param guest Guest label.
#' @param runtime,wasi,limits,callbacks Prepared capability objects.
#' @param store,linker Low-level store and linker handles for instantiation.
#' @param timeout_ms Optional wait timeout.
#' @usage
#' wt_runtime_spec()
#' wt_with_compiler(.x, strategy = c("auto", "cranelift", "winch"),
#'   opt_level = c("none", "speed", "speed_and_size"), parallel = TRUE)
#' wt_enable_features(.x, component_model = NULL, component_model_async = NULL,
#'   simd = NULL, relaxed_simd = NULL, relaxed_simd_deterministic = NULL,
#'   bulk_memory = NULL, multi_memory = NULL, memory64 = NULL, threads = NULL,
#'   exceptions = NULL, legacy_exceptions = NULL, gc = NULL)
#' wt_build_runtime(.x)
#' wt_wasi()
#' wt_limits()
#' wt_callbacks()
#' wt_app(source, kind = c("auto", "module", "component", "artifact"))
#' wt_prepare(.x)
#' wt_instantiate(.x, store, linker)
#' wt_imports(.x)
#' wt_exports(.x)
#' wt_bindings(.x)
#' wt_call(.x, export, ..., .args = NULL)
#' wt_call_async(.x, export, ..., .args = NULL)
#' wt_await(.x, timeout_ms = NULL)
#' wt_repl(.x = NULL, protocol = c("component", "stdio", "callback", "core", "mock"),
#'   eval_export = "eval", prompt = "> ", continuation = "+ ", guest = NULL, ...)
#' wt_webr_repl(source, runtime = NULL, wasi = NULL, limits = NULL,
#'   callbacks = NULL, protocol = c("component", "stdio"),
#'   eval_export = "webr:host/repl.eval")
#' @details
#' The canonical API is pipe-first: `object |> wt_verb(...)`. Composition verbs
#' return the same conceptual object. Terminal verbs return values, jobs, memory
#' reads, or array materializations.
#'
#' Async Wasm-to-R callbacks must be serviced by the adapter's platform callback
#' machinery, not by user-authored drain loops. The intended backend mirrors the
#' Rtinycc pattern: wake the R main thread with an input handler on POSIX and a
#' message pump on Windows, while worker threads block only on native
#' synchronization. `wt_drain_callbacks()` is therefore an advanced/test hook for
#' unusual tight native loops, not part of normal examples.
#'
#' `wt_imports()`, `wt_exports()`, and `wt_bindings()` inspect the structural
#' core Wasm ABI that the module itself declares. For core modules this reveals
#' value types, memories, tables, globals, and tags; it does not infer pointer,
#' string, array, handle, or ownership semantics unless a future WIT/custom
#' metadata layer supplies that policy.
#'
#' A sandbox REPL is not a Wasmtime built-in. It is a protocol supplied by the
#' guest, such as a component evaluator, stdio command loop, callback-backed
#' request/reply channel, or core-module memory ABI. `wt_webr_repl()` is a future
#' adapter for a webR guest and must not be implemented by evaluating code in
#' host R.
#' @return Runtime specs, runtime handles, WASI specs, limit specs, callback
#'   sets, app specs, prepared app handles, jobs, memory handles, REPL handles,
#'   or result objects depending on the verb.
#' @examples
#' rt <- wt_runtime_spec() |>
#'   wt_with_compiler("cranelift") |>
#'   wt_enable_features(component_model = TRUE, simd = TRUE) |>
#'   wt_build_runtime()
#'
#' wasi <- wt_wasi() |>
#'   wt_wasi_stdio(stdout = "capture", stderr = "capture")
#'
#' component <- wt_component("plugin.component.wasm") |>
#'   wt_with_runtime(rt) |>
#'   wt_with_wasi(wasi)
#'
#' \dontrun{
#' component |> wt_component_exports()
#' }
#'
#' # Component execution is a future WIT/value-conversion boundary.
NULL

# Rwasmtime pipe-first R API.
#
# This file intentionally contains lightweight R-side value objects and honest
# pending-backend boundaries. The Wasmtime backend belongs in src/rust and the
# generated Savvy adapter owns R-facing native handles and type conversion.

wt_new <- function(class, ...) {
  structure(list(...), class = c(class, "WtObject"))
}

wt_new_env <- function(class, ...) {
  env <- list2env(list(...), parent = emptyenv())
  class(env) <- c(class, "WtObject")
  env
}

wt_check <- function(x, class, name = ".x") {
  if (!inherits(x, class)) {
    stop(name, " must inherit from ", class, call. = FALSE)
  }
  invisible(x)
}

wt_choose <- function(x, choices, name) {
  if (missing(x) || is.null(x)) return(choices[[1L]])
  match.arg(x, choices)
}

wt_not_implemented <- function(feature) {
  msg <- paste0(feature, " is not implemented for this Rwasmtime build or API path")
  cond <- structure(
    list(message = msg, call = NULL, feature = feature),
    class = c("rwasmtime_not_implemented", "rwasmtime_error", "error", "condition")
  )
  stop(cond)
}

wt_bytes <- function(x) {
  if (is.null(x)) return(NULL)
  if (is.numeric(x) && length(x) == 1L && is.finite(x) && x >= 0) return(as.numeric(x))
  if (!is.character(x) || length(x) != 1L || is.na(x)) {
    stop("byte size must be a non-negative scalar number or size string", call. = FALSE)
  }
  value <- trimws(x)
  m <- regexec("^([0-9]+(?:\\.[0-9]+)?)\\s*([KMGTP]?i?B?)?$", value, ignore.case = TRUE, perl = TRUE)
  parts <- regmatches(value, m)[[1L]]
  if (!length(parts)) stop("invalid byte size: ", sQuote(x), call. = FALSE)
  n <- as.numeric(parts[[2L]])
  unit <- toupper(parts[[3L]])
  mult <- if (identical(unit, "")) {
    1
  } else {
    switch(
      unit,
      "B" = 1,
      "K" = 1000,
      "KB" = 1000,
      "KI" = 1024,
      "KIB" = 1024,
      "M" = 1000^2,
      "MB" = 1000^2,
      "MI" = 1024^2,
      "MIB" = 1024^2,
      "G" = 1000^3,
      "GB" = 1000^3,
      "GI" = 1024^3,
      "GIB" = 1024^3,
      "T" = 1000^4,
      "TB" = 1000^4,
      "TI" = 1024^4,
      "TIB" = 1024^4,
      stop("unsupported byte unit: ", sQuote(unit), call. = FALSE)
    )
  }
  n * mult
}

wt_set_non_null <- function(x, values) {
  for (nm in names(values)) {
    if (!is.null(values[[nm]])) x[[nm]] <- values[[nm]]
  }
  x
}

# Runtime ---------------------------------------------------------------------

#' @export
wt_runtime_spec <- function() {
  wt_new(
    "WtRuntimeSpec",
    compiler = list(strategy = "auto", opt_level = "speed", parallel = TRUE),
    features = list(
      component_model = TRUE,
      component_model_async = FALSE,
      simd = TRUE,
      relaxed_simd = FALSE,
      relaxed_simd_deterministic = FALSE,
      bulk_memory = TRUE,
      multi_memory = TRUE,
      memory64 = FALSE,
      threads = FALSE,
      exceptions = FALSE,
      legacy_exceptions = FALSE,
      gc = FALSE
    ),
    aot = list(cache = TRUE, cache_dir = NULL, artifact_dir = NULL),
    allocator = list(strategy = "on_demand", memory_limit = NULL, table_limit = NULL, instance_limit = NULL)
  )
}

#' @export
wt_with_compiler <- function(.x,
                             strategy = c("auto", "cranelift", "winch"),
                             opt_level = c("none", "speed", "speed_and_size"),
                             parallel = TRUE) {
  wt_check(.x, "WtRuntimeSpec")
  strategy <- wt_choose(strategy, c("auto", "cranelift", "winch"), "strategy")
  opt_level <- wt_choose(opt_level, c("none", "speed", "speed_and_size"), "opt_level")
  if (identical(strategy, "winch") && !identical(opt_level, "none")) {
    stop("winch compiler requires opt_level = 'none'", call. = FALSE)
  }
  .x$compiler <- list(
    strategy = strategy,
    opt_level = opt_level,
    parallel = isTRUE(parallel)
  )
  .x
}

#' @export
wt_enable_features <- function(.x,
                               component_model = NULL,
                               component_model_async = NULL,
                               simd = NULL,
                               relaxed_simd = NULL,
                               relaxed_simd_deterministic = NULL,
                               bulk_memory = NULL,
                               multi_memory = NULL,
                               memory64 = NULL,
                               threads = NULL,
                               exceptions = NULL,
                               legacy_exceptions = NULL,
                               gc = NULL) {
  wt_check(.x, "WtRuntimeSpec")
  .x$features <- wt_set_non_null(.x$features, list(
    component_model = component_model,
    component_model_async = component_model_async,
    simd = simd,
    relaxed_simd = relaxed_simd,
    relaxed_simd_deterministic = relaxed_simd_deterministic,
    bulk_memory = bulk_memory,
    multi_memory = multi_memory,
    memory64 = memory64,
    threads = threads,
    exceptions = exceptions,
    legacy_exceptions = legacy_exceptions,
    gc = gc
  ))
  if (isTRUE(.x$features$component_model_async) && !isTRUE(.x$features$component_model)) {
    stop("component_model_async requires component_model = TRUE", call. = FALSE)
  }
  if (isTRUE(.x$features$relaxed_simd) && !isTRUE(.x$features$simd)) {
    stop("relaxed_simd requires simd = TRUE", call. = FALSE)
  }
  if (isTRUE(.x$features$relaxed_simd_deterministic) && !isTRUE(.x$features$relaxed_simd)) {
    stop("relaxed_simd_deterministic requires relaxed_simd = TRUE", call. = FALSE)
  }
  if (isTRUE(.x$features$legacy_exceptions) && !isTRUE(.x$features$exceptions)) {
    stop("legacy_exceptions requires exceptions = TRUE", call. = FALSE)
  }
  .x
}

#' @export
wt_with_aot <- function(.x, cache = NULL, cache_dir = NULL, artifact_dir = NULL) {
  wt_check(.x, "WtRuntimeSpec")
  .x$aot <- wt_set_non_null(.x$aot, list(cache = cache, cache_dir = cache_dir, artifact_dir = artifact_dir))
  .x
}

#' @export
wt_with_allocator <- function(.x,
                              strategy = c("on_demand", "pooling"),
                              memory_limit = NULL,
                              table_limit = NULL,
                              instance_limit = NULL) {
  wt_check(.x, "WtRuntimeSpec")
  .x$allocator <- wt_set_non_null(.x$allocator, list(
    strategy = wt_choose(strategy, c("on_demand", "pooling"), "strategy"),
    memory_limit = if (!is.null(memory_limit)) wt_bytes(memory_limit) else NULL,
    table_limit = table_limit,
    instance_limit = instance_limit
  ))
  .x
}

rwasmtime_runtime_ptr <- function(spec) {
  if (!exists("rwasmtime_backend_status", mode = "function")) return(NULL)
  if (!identical(rwasmtime_backend_status(), "native")) return(NULL)
  rwasmtime_with_runtime_build_errors(
    RwasmtimeNativeRuntime$build(
      compiler_strategy = spec$compiler$strategy,
      opt_level = spec$compiler$opt_level,
      parallel = isTRUE(spec$compiler$parallel),
      component_model = isTRUE(spec$features$component_model),
      component_model_async = isTRUE(spec$features$component_model_async),
      simd = isTRUE(spec$features$simd),
      relaxed_simd = isTRUE(spec$features$relaxed_simd),
      relaxed_simd_deterministic = isTRUE(spec$features$relaxed_simd_deterministic),
      bulk_memory = isTRUE(spec$features$bulk_memory),
      multi_memory = isTRUE(spec$features$multi_memory),
      memory64 = isTRUE(spec$features$memory64),
      threads = isTRUE(spec$features$threads),
      exceptions = isTRUE(spec$features$exceptions),
      legacy_exceptions = isTRUE(spec$features$legacy_exceptions),
      gc = isTRUE(spec$features$gc)
    )
  )
}

rwasmtime_limit_arg <- function(limits, name) {
  if (inherits(limits, "WtLimits") && !is.null(limits[[name]])) as.numeric(limits[[name]]) else -1
}

rwasmtime_limit_args <- function(limits = NULL) {
  list(
    memory_bytes = rwasmtime_limit_arg(limits, "memory_bytes"),
    table_elements = rwasmtime_limit_arg(limits, "table_elements"),
    instances = rwasmtime_limit_arg(limits, "instances"),
    fuel = rwasmtime_limit_arg(limits, "fuel"),
    wall_time_ms = rwasmtime_limit_arg(limits, "wall_time_ms")
  )
}

rwasmtime_call_core <- function(runtime, module, export, args, limits = NULL) {
  do.call(runtime$ptr$call_core, c(list(module = as.character(module), export = as.character(export), args = args), rwasmtime_limit_args(limits)))
}

rwasmtime_call_core_module <- function(module, export, args, limits = NULL) {
  instance <- rwasmtime_instantiate_core_module(module, limits)
  instance$call_core(as.character(export), args)
}

rwasmtime_call_core_module_callbacks <- function(module, export, args, callbacks, limits = NULL) {
  instance <- rwasmtime_instantiate_core_module_callbacks(module, callbacks, limits)
  instance$call_core(as.character(export), args)
}

rwasmtime_call_core_module_wasi_p1 <- function(module, wasi, limits = NULL) {
  instance <- rwasmtime_instantiate_core_module_wasi_p1(module, wasi, limits)
  instance$call_core("_start", list())
  rwasmtime_wasi_result(instance$wasi_output(), wasi = wasi)
}

rwasmtime_call_core_module_wasi_p1_callbacks <- function(module, wasi, callbacks, limits = NULL) {
  instance <- rwasmtime_instantiate_core_module_wasi_p1_callbacks(module, wasi, callbacks, limits)
  instance$call_core("_start", list())
  rwasmtime_wasi_result(instance$wasi_output(), wasi = wasi)
}

rwasmtime_module_input <- function(input) {
  if (is.raw(input)) return(input)
  if (!is.character(input) || length(input) != 1L || is.na(input)) {
    stop("Wasm module input must be a character scalar or raw vector", call. = FALSE)
  }
  if (file.exists(input) && !dir.exists(input)) {
    size <- file.info(input)$size
    return(readBin(input, what = "raw", n = size))
  }
  input
}

rwasmtime_compile_core <- function(runtime, module) {
  rwasmtime_with_compile_errors(runtime$ptr$compile_core(rwasmtime_module_input(module)))
}

rwasmtime_deserialize_core <- function(runtime, bytes) {
  rwasmtime_with_compile_errors(runtime$ptr$deserialize_core(bytes))
}

rwasmtime_serialize_core_module <- function(artifact) {
  artifact$ptr$serialize()
}

rwasmtime_aot_metadata_path <- function(path) {
  paste0(path, ".rwasmtime.rds")
}

rwasmtime_aot_compatible <- function(metadata, runtime) {
  is.list(metadata) &&
    isTRUE(identical(metadata$format_version, 1L)) &&
    identical(metadata$rwasmtime_version, as.character(utils::packageVersion("Rwasmtime"))) &&
    is.character(metadata$wasmtime_version) && length(metadata$wasmtime_version) == 1L && nzchar(metadata$wasmtime_version) &&
    identical(metadata$target, R.version$platform) &&
    identical(metadata$compiler, runtime$spec$compiler) &&
    identical(metadata$features, runtime$spec$features)
}

rwasmtime_stop_condition <- function(message, class, parent = NULL, ...) {
  cond <- structure(
    c(list(message = message, call = NULL, parent = parent), list(...)),
    class = c(class, "rwasmtime_error", "error", "condition")
  )
  stop(cond)
}

rwasmtime_stop_aot_incompatible <- function(message, metadata = NULL) {
  rwasmtime_stop_condition(message, "rwasmtime_aot_incompatible", metadata = metadata)
}

rwasmtime_stop_compile_error <- function(err) {
  rwasmtime_stop_condition(conditionMessage(err), "rwasmtime_compile_error", parent = err)
}

rwasmtime_stop_link_error <- function(err) {
  rwasmtime_stop_condition(conditionMessage(err), "rwasmtime_link_error", parent = err)
}

rwasmtime_stop_instantiate_error <- function(err, class = "rwasmtime_instantiate_error") {
  rwasmtime_stop_condition(conditionMessage(err), class, parent = err)
}

rwasmtime_stop_unsupported_feature <- function(err, class = character()) {
  rwasmtime_stop_condition(conditionMessage(err), c("rwasmtime_unsupported_feature", class), parent = err)
}

rwasmtime_stop_callback_error <- function(err) {
  rwasmtime_stop_condition(conditionMessage(err), "rwasmtime_callback_error", parent = err)
}

rwasmtime_stop_trap <- function(err) {
  rwasmtime_stop_condition(conditionMessage(err), "rwasmtime_trap", parent = err)
}

rwasmtime_stop_timeout <- function(message, timeout_ms = NULL, job = NULL) {
  rwasmtime_stop_condition(message, "rwasmtime_timeout", timeout_ms = timeout_ms, job = job)
}

rwasmtime_validate_timeout_ms <- function(timeout_ms) {
  if (is.null(timeout_ms)) return(NULL)
  if (!is.numeric(timeout_ms) || length(timeout_ms) != 1L || is.na(timeout_ms) || !is.finite(timeout_ms) || timeout_ms < 0) {
    stop("timeout_ms must be NULL or a non-negative finite scalar", call. = FALSE)
  }
  as.numeric(timeout_ms)
}

rwasmtime_validate_limit_count <- function(value, message) {
  max_exact <- 9007199254740992
  if (!is.numeric(value) || length(value) != 1L || is.na(value) || !is.finite(value) || value < 0 || value != floor(value) || value > max_exact) {
    stop(message, call. = FALSE)
  }
  as.numeric(value)
}

rwasmtime_is_limit_error_message <- function(message) {
  grepl("all fuel consumed", message, fixed = TRUE) ||
    grepl("wall time limit exceeded", message, fixed = TRUE) ||
    grepl("memory limit exceeded", message, fixed = TRUE) ||
    grepl("table element limit exceeded", message, fixed = TRUE) ||
    grepl("resource limit exceeded", message, fixed = TRUE)
}

rwasmtime_is_unsupported_feature_message <- function(message) {
  feature_words <- grepl("feature", message, fixed = TRUE) ||
    grepl("proposal", message, fixed = TRUE) ||
    grepl("exceptions", message, fixed = TRUE) ||
    grepl("SIMD", message, ignore.case = TRUE)
  feature_words && (
    grepl("not supported", message, fixed = TRUE) ||
      grepl("not enabled", message, fixed = TRUE) ||
      grepl("required for", message, fixed = TRUE)
  )
}

rwasmtime_is_link_error_message <- function(message) {
  grepl("failed to link", message, fixed = TRUE) ||
    grepl("missing R callback", message, fixed = TRUE) ||
    grepl("callback import", message, fixed = TRUE) ||
    grepl("provided but not imported", message, fixed = TRUE) ||
    grepl("incompatible function signatures", message, fixed = TRUE) ||
    (grepl("expected", message, fixed = TRUE) && grepl("imports", message, fixed = TRUE)) ||
    grepl("unknown import", message, fixed = TRUE)
}

rwasmtime_limit_error_fields <- function(message) {
  memory <- regexec("requested ([0-9]+) bytes exceeds configured memory limit ([0-9]+) bytes", message, perl = TRUE)
  parts <- regmatches(message, memory)[[1L]]
  if (length(parts)) return(list(requested = as.numeric(parts[[2L]]), limit = as.numeric(parts[[3L]])))
  table <- regexec("requested ([0-9]+) elements exceeds configured table element limit ([0-9]+) elements", message, perl = TRUE)
  parts <- regmatches(message, table)[[1L]]
  if (length(parts)) return(list(requested = as.numeric(parts[[2L]]), limit = as.numeric(parts[[3L]])))
  list(requested = NULL, limit = NULL)
}

rwasmtime_with_limit_errors <- function(expr) {
  tryCatch(
    force(expr),
    error = function(err) {
      message <- conditionMessage(err)
      if (rwasmtime_is_limit_error_message(message)) {
        fields <- rwasmtime_limit_error_fields(message)
        rwasmtime_stop_limit_error(message, limit = fields$limit, requested = fields$requested, parent = err)
      }
      stop(err)
    }
  )
}

rwasmtime_with_runtime_build_errors <- function(expr) {
  tryCatch(
    force(expr),
    error = function(err) {
      if (inherits(err, "rwasmtime_error")) stop(err)
      if (rwasmtime_is_unsupported_feature_message(conditionMessage(err))) {
        rwasmtime_stop_unsupported_feature(err)
      }
      stop(err)
    }
  )
}

rwasmtime_with_compile_errors <- function(expr) {
  tryCatch(
    force(expr),
    error = function(err) {
      if (inherits(err, "rwasmtime_error")) stop(err)
      if (rwasmtime_is_unsupported_feature_message(conditionMessage(err))) {
        rwasmtime_stop_unsupported_feature(err, "rwasmtime_compile_error")
      }
      rwasmtime_stop_compile_error(err)
    }
  )
}

rwasmtime_with_instantiate_errors <- function(expr) {
  tryCatch(
    force(expr),
    error = function(err) {
      if (inherits(err, "rwasmtime_error")) stop(err)
      message <- conditionMessage(err)
      if (rwasmtime_is_limit_error_message(message)) {
        fields <- rwasmtime_limit_error_fields(message)
        rwasmtime_stop_limit_error(message, limit = fields$limit, requested = fields$requested, parent = err)
      }
      if (rwasmtime_is_unsupported_feature_message(message)) {
        rwasmtime_stop_unsupported_feature(err, "rwasmtime_instantiate_error")
      }
      if (rwasmtime_is_link_error_message(message)) {
        rwasmtime_stop_link_error(err)
      }
      if (grepl("R callback", message, fixed = TRUE)) {
        rwasmtime_stop_callback_error(err)
      }
      rwasmtime_stop_instantiate_error(err)
    }
  )
}

rwasmtime_with_wasm_call_errors <- function(expr) {
  tryCatch(
    force(expr),
    error = function(err) {
      message <- conditionMessage(err)
      is_wasm_call_error <- grepl("Wasm call `", message, fixed = TRUE) &&
        grepl("trapped or failed", message, fixed = TRUE)
      if (is_wasm_call_error && grepl("R callback", message, fixed = TRUE)) {
        rwasmtime_stop_callback_error(err)
      }
      if (rwasmtime_is_limit_error_message(message)) {
        fields <- rwasmtime_limit_error_fields(message)
        rwasmtime_stop_limit_error(message, limit = fields$limit, requested = fields$requested, parent = err)
      }
      if (is_wasm_call_error) {
        rwasmtime_stop_trap(err)
      }
      stop(err)
    }
  )
}

rwasmtime_with_callback_errors <- rwasmtime_with_wasm_call_errors

rwasmtime_instantiate_core <- function(runtime, module, limits = NULL) {
  rwasmtime_with_instantiate_errors(do.call(runtime$ptr$instantiate_core, c(list(module = as.character(module)), rwasmtime_limit_args(limits))))
}

rwasmtime_instantiate_core_module <- function(module, limits = NULL) {
  rwasmtime_with_instantiate_errors(do.call(module$ptr$instantiate, rwasmtime_limit_args(limits)))
}

rwasmtime_instantiate_core_callbacks <- function(runtime, module, callbacks, limits = NULL) {
  compiled <- rwasmtime_compile_core(runtime, module)
  rwasmtime_with_limit_errors(do.call(compiled$instantiate_callbacks, c(rwasmtime_core_callback_parts(callbacks), rwasmtime_limit_args(limits))))
}

rwasmtime_call_core_callbacks <- function(runtime, module, export, args, callbacks) {
  instance <- rwasmtime_instantiate_core_callbacks(runtime, module, callbacks)
  instance$call_core(as.character(export), args)
}

rwasmtime_wrap_core_callback <- function(entry, key) {
  policy <- entry$policy
  calls <- 0L
  depth <- 0L
  force(entry)
  force(key)
  function(...) {
    if (!is.null(policy$max_calls) && calls >= policy$max_calls) {
      stop("callback call limit exceeded for `", key, "`", call. = FALSE)
    }
    if (!isTRUE(policy$reentrant) && depth > 0L) {
      stop("reentrant callback call rejected for `", key, "`", call. = FALSE)
    }
    if (depth >= policy$max_depth) {
      stop("callback max_depth exceeded for `", key, "`", call. = FALSE)
    }
    calls <<- calls + 1L
    depth <<- depth + 1L
    on.exit({ depth <<- depth - 1L }, add = TRUE)
    start <- unname(proc.time()[["elapsed"]])
    out <- entry$fun(...)
    elapsed_ms <- (unname(proc.time()[["elapsed"]]) - start) * 1000
    if (!is.null(policy$timeout_ms) && elapsed_ms > policy$timeout_ms) {
      stop("callback timeout exceeded for `", key, "`", call. = FALSE)
    }
    out
  }
}

rwasmtime_core_callback_parts <- function(callbacks) {
  entries <- callbacks$entries
  if (!length(entries)) {
    return(list(callback_modules = character(), callback_names = character(), callback_abis = character(), callback_functions = list()))
  }
  allowed <- vapply(entries, function(entry) entry$abi %in% c("core", "core_memory_request"), logical(1), USE.NAMES = FALSE)
  if (any(!allowed)) {
    stop("native core callback imports currently require callbacks declared with abi = 'core' or 'core_memory_request'", call. = FALSE)
  }
  keys <- names(entries)
  list(
    callback_modules = vapply(entries, function(entry) entry$module, character(1), USE.NAMES = FALSE),
    callback_names = vapply(entries, function(entry) entry$name, character(1), USE.NAMES = FALSE),
    callback_abis = vapply(entries, function(entry) entry$abi, character(1), USE.NAMES = FALSE),
    callback_functions = Map(rwasmtime_wrap_core_callback, entries, keys)
  )
}

rwasmtime_instantiate_core_module_callbacks <- function(module, callbacks, limits = NULL) {
  rwasmtime_with_instantiate_errors(do.call(module$ptr$instantiate_callbacks, c(rwasmtime_core_callback_parts(callbacks), rwasmtime_limit_args(limits))))
}

rwasmtime_wasi_stdin_input <- function(wasi) {
  if (identical(wasi$stdin, "string")) {
    if (is.null(wasi$input)) stop("stdin='string' requires input", call. = FALSE)
    return(charToRaw(wasi$input))
  }
  if (!identical(wasi$stdin, "file")) return(NULL)
  path <- wasi$stdin_file
  if (!file.exists(path)) stop("stdin_file does not exist: ", path, call. = FALSE)
  size <- file.info(path)$size
  readBin(path, what = "raw", n = size)
}

rwasmtime_wasi_call_parts <- function(wasi) {
  preopens <- wasi$preopens
  preopen_guest <- vapply(preopens, function(x) x$guest, character(1), USE.NAMES = FALSE)
  preopen_host <- vapply(preopens, function(x) x$host, character(1), USE.NAMES = FALSE)
  preopen_readonly <- vapply(preopens, function(x) isTRUE(x$readonly), logical(1), USE.NAMES = FALSE)
  env_names <- names(wasi$env)
  if (is.null(env_names)) env_names <- character()
  list(
    args = as.character(wasi$args),
    env_names = as.character(env_names),
    env_values = as.character(unname(wasi$env)),
    preopen_guest = as.character(preopen_guest),
    preopen_host = as.character(preopen_host),
    preopen_readonly = preopen_readonly,
    stdin = if (identical(wasi$stdin, "file")) "string" else wasi$stdin,
    stdout = if (identical(wasi$stdout, "file")) "capture" else wasi$stdout,
    stderr = if (identical(wasi$stderr, "file")) "capture" else wasi$stderr,
    input = rwasmtime_wasi_stdin_input(wasi)
  )
}

rwasmtime_instantiate_core_module_wasi_p1 <- function(module, wasi, limits = NULL) {
  rwasmtime_with_instantiate_errors(do.call(module$ptr$instantiate_wasi_p1, c(rwasmtime_wasi_call_parts(wasi), rwasmtime_limit_args(limits))))
}

rwasmtime_instantiate_core_module_wasi_p1_callbacks <- function(module, wasi, callbacks, limits = NULL) {
  rwasmtime_with_instantiate_errors(
    do.call(
      module$ptr$instantiate_wasi_p1_callbacks,
      c(rwasmtime_core_callback_parts(callbacks), rwasmtime_wasi_call_parts(wasi), rwasmtime_limit_args(limits))
    )
  )
}

rwasmtime_call_instance_core <- function(instance, export, args) {
  instance$ptr$call_core(as.character(export), args)
}

rwasmtime_raw_to_text <- function(bytes) {
  if (!length(bytes)) return("")
  tryCatch(rawToChar(bytes), error = function(err) NA_character_)
}

rwasmtime_wasi_result <- function(out, wasi = NULL) {
  stdout_raw <- out$stdout_raw
  stderr_raw <- out$stderr_raw
  if (!is.null(wasi) && identical(wasi$stdout, "file")) {
    writeBin(stdout_raw, wasi$stdout_file, useBytes = TRUE)
  }
  if (!is.null(wasi) && identical(wasi$stderr, "file")) {
    writeBin(stderr_raw, wasi$stderr_file, useBytes = TRUE)
  }
  wt_new(
    "WtWasiResult",
    stdout_raw = stdout_raw,
    stderr_raw = stderr_raw,
    stdout = rwasmtime_raw_to_text(stdout_raw),
    stderr = rwasmtime_raw_to_text(stderr_raw),
    stdout_file = if (!is.null(wasi) && identical(wasi$stdout, "file")) wasi$stdout_file else NULL,
    stderr_file = if (!is.null(wasi) && identical(wasi$stderr, "file")) wasi$stderr_file else NULL
  )
}

rwasmtime_instance_wasi_output <- function(instance, wasi = NULL) {
  rwasmtime_wasi_result(instance$ptr$wasi_output(), wasi = wasi)
}

rwasmtime_memory_instance <- function(memory) {
  owner <- memory$owner
  if ((inherits(owner, "WtSession") || inherits(owner, "WtInstance")) && identical(owner$backend, "native") && !is.null(owner$ptr)) {
    return(owner$ptr)
  }
  NULL
}

rwasmtime_owner_limits <- function(owner) {
  if (inherits(owner, "WtSession")) return(owner$app$spec$limits)
  if (inherits(owner, "WtInstance")) return(owner$store$limits)
  NULL
}

rwasmtime_memory_limit_bytes <- function(owner) {
  limits <- rwasmtime_owner_limits(owner)
  if (inherits(limits, "WtLimits") && !is.null(limits$memory_bytes)) as.numeric(limits$memory_bytes) else NULL
}

rwasmtime_stop_limit_error <- function(message, limit = NULL, requested = NULL, parent = NULL) {
  cond <- structure(
    list(message = message, call = NULL, limit = limit, requested = requested, parent = parent),
    class = c("rwasmtime_limit_error", "rwasmtime_error", "error", "condition")
  )
  stop(cond)
}

rwasmtime_check_memory_limit <- function(owner, requested_bytes, operation) {
  limit <- rwasmtime_memory_limit_bytes(owner)
  if (!is.null(limit) && requested_bytes > limit) {
    rwasmtime_stop_limit_error(
      paste0(operation, " exceeds configured memory limit"),
      limit = limit,
      requested = requested_bytes
    )
  }
}

rwasmtime_check_memory_object_limit <- function(memory, requested_bytes = NULL, operation) {
  limit <- rwasmtime_memory_limit_bytes(memory$owner)
  if (is.null(limit)) return(invisible(NULL))
  current_bytes <- rwasmtime_memory_size(memory) * 65536
  if (current_bytes > limit) {
    rwasmtime_stop_limit_error(
      paste0(operation, " current memory exceeds configured memory limit"),
      limit = limit,
      requested = current_bytes
    )
  }
  if (!is.null(requested_bytes) && requested_bytes > limit) {
    rwasmtime_stop_limit_error(
      paste0(operation, " exceeds configured memory limit"),
      limit = limit,
      requested = requested_bytes
    )
  }
  invisible(NULL)
}

rwasmtime_memory_size <- function(memory) {
  rwasmtime_memory_instance(memory)$memory_size(as.character(memory$name))
}

rwasmtime_memory_grow <- function(memory, pages) {
  current_pages <- rwasmtime_memory_size(memory)
  rwasmtime_check_memory_object_limit(memory, (current_pages + as.numeric(pages)) * 65536, "wt_memory_grow")
  rwasmtime_memory_instance(memory)$memory_grow(as.character(memory$name), as.numeric(pages))
}

rwasmtime_memory_read_raw <- function(memory, ptr, length) {
  rwasmtime_memory_instance(memory)$memory_read(as.character(memory$name), as.numeric(ptr), as.numeric(length))
}

rwasmtime_memory_write_raw <- function(memory, ptr, value) {
  rwasmtime_check_memory_object_limit(memory, as.numeric(ptr) + length(value), "wt_memory_write")
  rwasmtime_memory_instance(memory)$memory_write(as.character(memory$name), as.numeric(ptr), value)
}

rwasmtime_dtype_size <- function(dtype) {
  switch(
    dtype,
    u8 = 1L,
    i32 = 4L,
    u32 = 4L,
    f32 = 4L,
    i64 = 8L,
    u64 = 8L,
    f64 = 8L,
    v128 = 16L,
    stop("unsupported memory dtype: ", dtype, call. = FALSE)
  )
}

rwasmtime_unpack_memory <- function(bytes, dtype, n) {
  if (identical(dtype, "u8")) return(bytes)
  if (identical(dtype, "v128")) wt_not_implemented("wt_memory_read dtype=v128")
  if (!is.raw(bytes)) stop("internal memory payload must be raw", call. = FALSE)
  if (!length(bytes)) {
    if (dtype %in% c("i64", "u64")) return(character())
    return(numeric())
  }
  if (dtype %in% c("i32", "u32")) return(rwasmtime_unpack_u32(bytes, signed = identical(dtype, "i32")))
  if (identical(dtype, "f32")) return(readBin(bytes, what = "numeric", n = n, size = 4L, endian = "little"))
  if (identical(dtype, "f64")) return(readBin(bytes, what = "numeric", n = n, size = 8L, endian = "little"))
  if (identical(dtype, "i64")) return(rwasmtime_unpack_i64(bytes))
  if (identical(dtype, "u64")) return(rwasmtime_unpack_u64(bytes))
  stop("unsupported memory dtype: ", dtype, call. = FALSE)
}

rwasmtime_pack_memory <- function(value, dtype) {
  if (identical(dtype, "u8")) {
    if (!is.raw(value)) stop("value must be a raw vector when dtype = 'u8'", call. = FALSE)
    return(value)
  }
  if (identical(dtype, "v128")) wt_not_implemented("wt_memory_write dtype=v128")
  if (dtype %in% c("i32", "u32")) return(rwasmtime_pack_u32(value, signed = identical(dtype, "i32"), dtype = dtype))
  if (identical(dtype, "f32")) return(rwasmtime_pack_float(value, dtype = "f32", size = 4L))
  if (identical(dtype, "f64")) return(rwasmtime_pack_float(value, dtype = "f64", size = 8L))
  if (identical(dtype, "i64")) return(rwasmtime_pack_i64(value))
  if (identical(dtype, "u64")) return(rwasmtime_pack_u64(value))
  stop("unsupported memory dtype: ", dtype, call. = FALSE)
}

rwasmtime_array_values <- function(value, layout) {
  d <- dim(value)
  if (is.null(d) || identical(layout, "column-major") || identical(layout, "contiguous")) return(as.vector(value))
  if (identical(layout, "row-major") && length(d) == 2L) return(as.vector(t(value)))
  wt_not_implemented("row-major arrays with more than two dimensions")
}

rwasmtime_array_length <- function(values, dtype) {
  if (identical(dtype, "u8")) length(values) else length(as.vector(values))
}

rwasmtime_shape_array <- function(value, dim, layout) {
  if (is.null(dim)) return(value)
  if (prod(dim) != length(value)) stop("dim product must match array element count", call. = FALSE)
  if (identical(layout, "row-major")) {
    if (length(dim) != 2L) wt_not_implemented("row-major arrays with more than two dimensions")
    return(matrix(value, nrow = dim[[1L]], ncol = dim[[2L]], byrow = TRUE))
  }
  dim(value) <- dim
  value
}

rwasmtime_unpack_u32 <- function(bytes, signed) {
  if (length(bytes) %% 4L != 0L) stop("i32/u32 memory payload length must be a multiple of 4", call. = FALSE)
  b <- matrix(as.integer(bytes), nrow = 4L)
  out <- b[1L, ] + b[2L, ] * 256 + b[3L, ] * 65536 + b[4L, ] * 16777216
  if (isTRUE(signed)) out <- ifelse(out >= 2147483648, out - 4294967296, out)
  as.numeric(out)
}

rwasmtime_pack_float <- function(value, dtype, size) {
  if (!is.numeric(value) && !is.integer(value) && !is.logical(value)) {
    stop("value must be numeric or logical for dtype = '", dtype, "'", call. = FALSE)
  }
  writeBin(as.numeric(value), raw(), size = size, endian = "little")
}

rwasmtime_pack_u32 <- function(value, signed, dtype) {
  value <- as.numeric(value)
  if (anyNA(value) || any(!is.finite(value)) || any(value != floor(value))) {
    stop("value must contain whole finite numbers for dtype = '", dtype, "'", call. = FALSE)
  }
  min_value <- if (isTRUE(signed)) -2147483648 else 0
  max_value <- if (isTRUE(signed)) 2147483647 else 4294967295
  if (any(value < min_value | value > max_value)) {
    stop("value is outside the representable range for dtype = '", dtype, "'", call. = FALSE)
  }
  unsigned <- ifelse(value < 0, value + 4294967296, value)
  b0 <- unsigned %% 256
  b1 <- floor(unsigned / 256) %% 256
  b2 <- floor(unsigned / 65536) %% 256
  b3 <- floor(unsigned / 16777216) %% 256
  as.raw(as.vector(rbind(b0, b1, b2, b3)))
}

rwasmtime_unpack_u64 <- function(bytes) {
  if (length(bytes) %% 8L != 0L) stop("u64 memory payload length must be a multiple of 8", call. = FALSE)
  b <- matrix(as.integer(bytes), nrow = 8L)
  unname(apply(b, 2L, function(x) rwasmtime_u64_bytes_to_decimal(as.raw(x)), simplify = TRUE))
}

rwasmtime_unpack_i64 <- function(bytes) {
  if (length(bytes) %% 8L != 0L) stop("i64 memory payload length must be a multiple of 8", call. = FALSE)
  b <- matrix(as.integer(bytes), nrow = 8L)
  unname(apply(b, 2L, function(x) {
    if (x[8L] >= 128L) {
      paste0("-", rwasmtime_u64_bytes_to_decimal(rwasmtime_negate_64_bytes(as.raw(x))))
    } else {
      rwasmtime_u64_bytes_to_decimal(as.raw(x))
    }
  }, simplify = TRUE))
}

rwasmtime_pack_u64 <- function(value) {
  values <- rwasmtime_integer_strings(value, signed = FALSE, dtype = "u64")
  do.call(c, lapply(values, rwasmtime_decimal_to_u64_bytes))
}

rwasmtime_pack_i64 <- function(value) {
  values <- rwasmtime_integer_strings(value, signed = TRUE, dtype = "i64")
  do.call(c, lapply(values, function(x) {
    negative <- startsWith(x, "-")
    magnitude <- if (negative) substring(x, 2L) else x
    bytes <- rwasmtime_decimal_to_u64_bytes(magnitude)
    if (negative) bytes <- rwasmtime_negate_64_bytes(bytes)
    bytes
  }))
}

rwasmtime_integer_strings <- function(value, signed, dtype) {
  if (is.character(value)) {
    out <- trimws(value)
  } else {
    numeric <- as.numeric(value)
    if (anyNA(numeric) || any(!is.finite(numeric)) || any(numeric != floor(numeric))) {
      stop("value must contain whole finite numbers or decimal strings for dtype = '", dtype, "'", call. = FALSE)
    }
    if (any(abs(numeric) >= 9007199254740992)) {
      stop("numeric ", dtype, " values at or above 2^53 must be supplied as decimal strings to avoid precision loss", call. = FALSE)
    }
    out <- format(numeric, scientific = FALSE, trim = TRUE)
  }
  pattern <- if (isTRUE(signed)) "^-?[0-9]+$" else "^[0-9]+$"
  if (any(!grepl(pattern, out))) {
    stop("value must contain decimal integer strings for dtype = '", dtype, "'", call. = FALSE)
  }
  out <- vapply(out, rwasmtime_normalize_decimal, character(1), USE.NAMES = FALSE)
  if (isTRUE(signed)) {
    too_small <- startsWith(out, "-") & vapply(substring(out, 2L), rwasmtime_decimal_gt, logical(1), "9223372036854775808", USE.NAMES = FALSE)
    too_large <- !startsWith(out, "-") & vapply(out, rwasmtime_decimal_gt, logical(1), "9223372036854775807", USE.NAMES = FALSE)
    if (any(too_small | too_large)) stop("value is outside the representable range for dtype = 'i64'", call. = FALSE)
  } else {
    too_large <- vapply(out, rwasmtime_decimal_gt, logical(1), "18446744073709551615", USE.NAMES = FALSE)
    if (any(too_large)) stop("value is outside the representable range for dtype = 'u64'", call. = FALSE)
  }
  out
}

rwasmtime_normalize_decimal <- function(x) {
  negative <- startsWith(x, "-")
  if (negative) x <- substring(x, 2L)
  x <- sub("^0+", "", x)
  if (!nzchar(x)) x <- "0"
  if (negative && !identical(x, "0")) paste0("-", x) else x
}

rwasmtime_decimal_gt <- function(a, b) {
  a <- rwasmtime_normalize_decimal(a)
  b <- rwasmtime_normalize_decimal(b)
  if (nchar(a) != nchar(b)) return(nchar(a) > nchar(b))
  a > b
}

rwasmtime_decimal_to_u64_bytes <- function(value) {
  value <- rwasmtime_normalize_decimal(value)
  bytes <- integer(8L)
  i <- 1L
  while (!identical(value, "0")) {
    div <- rwasmtime_decimal_divmod_small(value, 256L)
    if (i > 8L) stop("decimal value exceeds 64 bits", call. = FALSE)
    bytes[[i]] <- div$remainder
    value <- div$quotient
    i <- i + 1L
  }
  as.raw(bytes)
}

rwasmtime_decimal_divmod_small <- function(value, divisor) {
  digits <- as.integer(strsplit(value, "", fixed = TRUE)[[1L]])
  quotient <- integer(length(digits))
  carry <- 0L
  for (i in seq_along(digits)) {
    acc <- carry * 10L + digits[[i]]
    quotient[[i]] <- acc %/% divisor
    carry <- acc %% divisor
  }
  quotient <- paste0(quotient, collapse = "")
  quotient <- sub("^0+", "", quotient)
  if (!nzchar(quotient)) quotient <- "0"
  list(quotient = quotient, remainder = carry)
}

rwasmtime_u64_bytes_to_decimal <- function(bytes) {
  out <- "0"
  for (byte in rev(as.integer(bytes))) {
    out <- rwasmtime_decimal_mul_add(out, 256L, byte)
  }
  out
}

rwasmtime_decimal_mul_add <- function(value, multiplier, addend) {
  digits <- rev(as.integer(strsplit(value, "", fixed = TRUE)[[1L]]))
  carry <- addend
  out <- integer(length(digits))
  for (i in seq_along(digits)) {
    acc <- digits[[i]] * multiplier + carry
    out[[i]] <- acc %% 10L
    carry <- acc %/% 10L
  }
  while (carry > 0L) {
    out <- c(out, carry %% 10L)
    carry <- carry %/% 10L
  }
  paste0(rev(out), collapse = "")
}

rwasmtime_negate_64_bytes <- function(bytes) {
  out <- 255L - as.integer(bytes)
  carry <- 1L
  for (i in seq_len(8L)) {
    acc <- out[[i]] + carry
    out[[i]] <- acc %% 256L
    carry <- acc %/% 256L
  }
  as.raw(out)
}

rwasmtime_scalar_string <- function(x, name, allow_null = FALSE) {
  if (is.null(x) && isTRUE(allow_null)) return(NULL)
  if (!is.character(x) || length(x) != 1L || is.na(x) || !nzchar(x)) {
    stop(name, " must be a non-empty string", call. = FALSE)
  }
  x
}

rwasmtime_scalar_whole_nonnegative <- function(x, name, allow_null = FALSE) {
  if (is.null(x) && isTRUE(allow_null)) return(NULL)
  if (!is.numeric(x) || length(x) != 1L || is.na(x) || !is.finite(x) || x < 0 || x != floor(x)) {
    stop(name, " must be a non-negative whole number", call. = FALSE)
  }
  as.numeric(x)
}

rwasmtime_core_repl_options <- function(dots) {
  get <- function(name, default = NULL) {
    if (!is.null(dots[[name]])) dots[[name]] else default
  }
  string_option <- function(name, default = NULL) {
    value <- get(name, default)
    if (identical(value, FALSE)) value <- NULL
    rwasmtime_scalar_string(value, name, allow_null = TRUE)
  }
  alloc_export <- string_option("alloc_export", "alloc")
  result_ptr_export <- string_option("result_ptr_export", "result_ptr")
  result_len_export <- string_option("result_len_export", "result_len")
  list(
    memory = rwasmtime_scalar_string(get("memory", get("memory_name", "memory")), "memory"),
    alloc_export = alloc_export,
    input_ptr = rwasmtime_scalar_whole_nonnegative(get("input_ptr"), "input_ptr", allow_null = TRUE),
    result_ptr_export = result_ptr_export,
    result_len_export = result_len_export,
    value_ptr_export = string_option("value_ptr_export", result_ptr_export),
    value_len_export = string_option("value_len_export", result_len_export),
    stdout_ptr_export = string_option("stdout_ptr_export"),
    stdout_len_export = string_option("stdout_len_export"),
    stderr_ptr_export = string_option("stderr_ptr_export"),
    stderr_len_export = string_option("stderr_len_export"),
    error_ptr_export = string_option("error_ptr_export"),
    error_len_export = string_option("error_len_export"),
    status_export = string_option("status_export"),
    complete_export = string_option("complete_export"),
    free_export = string_option("free_export"),
    status_ok = as.integer(get("status_ok", 0L))
  )
}

rwasmtime_core_repl_session <- function(.x) {
  if (inherits(.x, "WtSession")) return(.x)
  if (inherits(.x, "WtPreparedApp")) return(wt_new_session(.x))
  NULL
}

rwasmtime_core_repl_eval <- function(repl, code) {
  session <- repl$session
  options <- repl$protocol_options
  if (!inherits(session, "WtSession") || !identical(session$backend, "native") || is.null(session$ptr)) {
    wt_not_implemented("wt_repl_send protocol=core")
  }
  code <- paste(as.character(code), collapse = "\n")
  bytes <- charToRaw(enc2utf8(code))
  n <- length(bytes)
  ptr <- if (!is.null(options$alloc_export)) {
    wt_call(session, options$alloc_export, n)
  } else {
    options$input_ptr
  }
  ptr <- rwasmtime_scalar_whole_nonnegative(ptr, "core REPL input pointer")
  if (is.null(ptr)) stop("core REPL requires alloc_export or input_ptr", call. = FALSE)

  mem <- session |> wt_memory(options$memory)
  mem <- mem |> wt_memory_write(ptr = ptr, value = bytes, dtype = "u8")
  eval_status <- wt_call(session, repl$eval_export, ptr, n)
  if (!is.null(eval_status) && !identical(as.integer(eval_status), options$status_ok)) {
    stop("core REPL eval export returned non-ok transport status: ", eval_status, call. = FALSE)
  }

  read_text_pair <- function(ptr_export, len_export, missing = character()) {
    if (is.null(ptr_export) || is.null(len_export)) return(missing)
    text_len <- rwasmtime_scalar_whole_nonnegative(
      wt_call(session, len_export),
      paste0("core REPL ", len_export, " result length")
    )
    if (identical(text_len, 0)) return("")
    text_ptr <- rwasmtime_scalar_whole_nonnegative(
      wt_call(session, ptr_export),
      paste0("core REPL ", ptr_export, " result pointer")
    )
    rawToChar(mem |> wt_memory_read(ptr = text_ptr, length = text_len, dtype = "u8"))
  }

  value <- read_text_pair(options$value_ptr_export, options$value_len_export, missing = "")
  stdout <- read_text_pair(options$stdout_ptr_export, options$stdout_len_export, missing = character())
  stderr <- read_text_pair(options$stderr_ptr_export, options$stderr_len_export, missing = character())
  error <- read_text_pair(options$error_ptr_export, options$error_len_export, missing = NULL)
  status <- if (!is.null(options$status_export)) {
    as.integer(wt_call(session, options$status_export))
  } else if (is.null(eval_status)) {
    options$status_ok
  } else {
    as.integer(eval_status)
  }
  complete <- if (!is.null(options$complete_export)) {
    !identical(as.integer(wt_call(session, options$complete_export)), 0L)
  } else {
    TRUE
  }

  if (!is.null(options$free_export) && !is.null(options$alloc_export)) {
    invisible(wt_call(session, options$free_export, ptr, n))
  }

  wt_new(
    "WtReplResult",
    input = code,
    stdout = stdout,
    stderr = stderr,
    value = value,
    error = error,
    status = status,
    complete = complete
  )
}

rwasmtime_run_wasi_p1 <- function(runtime, module, wasi, limits = NULL) {
  out <- rwasmtime_with_limit_errors(do.call(runtime$ptr$run_wasi_p1, c(list(module = as.character(module)), rwasmtime_wasi_call_parts(wasi), rwasmtime_limit_args(limits))))
  rwasmtime_wasi_result(out, wasi = wasi)
}

#' @export
wt_build_runtime <- function(.x) {
  wt_check(.x, "WtRuntimeSpec")
  ptr <- rwasmtime_runtime_ptr(.x)
  wt_new("WtRuntime", spec = .x, ptr = ptr, backend = if (is.null(ptr)) "pending" else "native")
}

rwasmtime_feature_line <- function(features) {
  shown <- c("component_model", "simd", "relaxed_simd", "memory64", "threads", "exceptions", "legacy_exceptions")
  shown <- shown[shown %in% names(features)]
  paste0(shown, "=", vapply(features[shown], as.character, character(1)), collapse = " ")
}

#' @export
print.WtRuntimeSpec <- function(x, ...) {
  cat("<WtRuntimeSpec>\n", sep = "")
  cat("  compiler: ", x$compiler$strategy, " opt=", x$compiler$opt_level,
      " parallel=", x$compiler$parallel, "\n", sep = "")
  cat("  features: ", rwasmtime_feature_line(x$features), "\n", sep = "")
  invisible(x)
}

#' @export
print.WtRuntime <- function(x, ...) {
  backend <- if (!is.null(x$backend)) x$backend else if (is.null(x$ptr)) "pending" else "native"
  cat("<WtRuntime> backend=", backend, "\n", sep = "")
  cat("  compiler: ", x$spec$compiler$strategy, " opt=", x$spec$compiler$opt_level,
      " parallel=", x$spec$compiler$parallel, "\n", sep = "")
  cat("  features: ", rwasmtime_feature_line(x$spec$features), "\n", sep = "")
  invisible(x)
}

# WASI ------------------------------------------------------------------------

#' @export
wt_wasi <- function() {
  wt_new(
    "WtWasi",
    args = character(),
    env = character(),
    preopens = list(),
    stdin = "empty",
    stdout = "capture",
    stderr = "capture",
    network = FALSE,
    clocks = FALSE,
    random = FALSE
  )
}

#' @export
wt_wasi_args <- function(.x, ...) {
  wt_check(.x, "WtWasi")
  .x$args <- c(.x$args, as.character(unlist(list(...), use.names = FALSE)))
  .x
}

#' @export
wt_wasi_env <- function(.x, ...) {
  wt_check(.x, "WtWasi")
  env <- c(...)
  if (!length(env)) return(.x)
  if (is.null(names(env)) || any(!nzchar(names(env)))) {
    stop("WASI env entries must be named", call. = FALSE)
  }
  if (any(grepl("=", names(env), fixed = TRUE))) {
    stop("WASI env names must not contain '='", call. = FALSE)
  }
  .x$env <- c(.x$env, as.character(env))
  .x
}

#' @export
wt_wasi_preopen <- function(.x, guest, host, readonly = TRUE) {
  wt_check(.x, "WtWasi")
  if (!is.character(guest) || length(guest) != 1L || !nzchar(guest)) stop("guest must be a path string", call. = FALSE)
  if (!startsWith(guest, "/")) stop("guest must be an absolute WASI path", call. = FALSE)
  if (!is.character(host) || length(host) != 1L || !nzchar(host)) stop("host must be a path string", call. = FALSE)
  .x$preopens[[length(.x$preopens) + 1L]] <- list(guest = guest, host = host, readonly = isTRUE(readonly))
  .x
}

rwasmtime_scalar_character <- function(x, name, allow_empty = FALSE) {
  if (!is.character(x) || length(x) != 1L || is.na(x) || (!allow_empty && !nzchar(x))) {
    stop(name, " must be a ", if (allow_empty) "single non-NA string" else "non-empty string", call. = FALSE)
  }
  x
}

#' @export
wt_wasi_stdio <- function(.x,
                          stdin = NULL,
                          stdout = NULL,
                          stderr = NULL,
                          input = NULL,
                          stdin_file = NULL,
                          stdout_file = NULL,
                          stderr_file = NULL) {
  wt_check(.x, "WtWasi")
  stdin <- if (is.null(stdin)) .x$stdin else match.arg(stdin, c("empty", "inherit", "string", "file"))
  stdout <- if (is.null(stdout)) .x$stdout else match.arg(stdout, c("capture", "inherit", "file", "discard"))
  stderr <- if (is.null(stderr)) .x$stderr else match.arg(stderr, c("capture", "inherit", "file", "discard"))
  if (!is.null(input)) input <- rwasmtime_scalar_character(input, "input", allow_empty = TRUE)
  if (!is.null(stdin_file)) stdin_file <- rwasmtime_scalar_character(stdin_file, "stdin_file")
  if (!is.null(stdout_file)) stdout_file <- rwasmtime_scalar_character(stdout_file, "stdout_file")
  if (!is.null(stderr_file)) stderr_file <- rwasmtime_scalar_character(stderr_file, "stderr_file")

  if (identical(stdin, "string")) {
    input <- if (!is.null(input)) input else if (identical(.x$stdin, "string")) .x$input else NULL
    if (is.null(input)) stop("stdin='string' requires input", call. = FALSE)
  } else {
    if (!is.null(input)) stop("input is only valid with stdin='string'", call. = FALSE)
    input <- NULL
  }

  if (identical(stdin, "file")) {
    stdin_file <- if (!is.null(stdin_file)) stdin_file else if (identical(.x$stdin, "file")) .x$stdin_file else NULL
    if (is.null(stdin_file)) stop("stdin='file' requires stdin_file", call. = FALSE)
  } else {
    if (!is.null(stdin_file)) stop("stdin_file is only valid with stdin='file'", call. = FALSE)
    stdin_file <- NULL
  }

  if (identical(stdout, "file")) {
    stdout_file <- if (!is.null(stdout_file)) stdout_file else if (identical(.x$stdout, "file")) .x$stdout_file else NULL
    if (is.null(stdout_file)) stop("stdout='file' requires stdout_file", call. = FALSE)
  } else {
    if (!is.null(stdout_file)) stop("stdout_file is only valid with stdout='file'", call. = FALSE)
    stdout_file <- NULL
  }

  if (identical(stderr, "file")) {
    stderr_file <- if (!is.null(stderr_file)) stderr_file else if (identical(.x$stderr, "file")) .x$stderr_file else NULL
    if (is.null(stderr_file)) stop("stderr='file' requires stderr_file", call. = FALSE)
  } else {
    if (!is.null(stderr_file)) stop("stderr_file is only valid with stderr='file'", call. = FALSE)
    stderr_file <- NULL
  }

  if (identical(stdout, "file") && identical(stderr, "file")) {
    stdout_norm <- normalizePath(stdout_file, winslash = "/", mustWork = FALSE)
    stderr_norm <- normalizePath(stderr_file, winslash = "/", mustWork = FALSE)
    if (identical(stdout_norm, stderr_norm)) {
      stop("stdout_file and stderr_file must be different until native streaming file sinks are implemented", call. = FALSE)
    }
  }
  .x$stdin <- stdin
  .x$stdout <- stdout
  .x$stderr <- stderr
  .x$input <- input
  .x$stdin_file <- stdin_file
  .x$stdout_file <- stdout_file
  .x$stderr_file <- stderr_file
  .x
}

#' @export
wt_wasi_network <- function(.x, enabled = FALSE) {
  wt_check(.x, "WtWasi")
  .x$network <- isTRUE(enabled)
  .x
}

rwasmtime_unset <- function(x) if (is.null(x)) "unset" else as.character(x)

#' @export
print.WtWasi <- function(x, ...) {
  cat("<WtWasi> args=", length(x$args), " env=", length(x$env), " preopens=", length(x$preopens), "\n", sep = "")
  cat("  stdio: stdin=", x$stdin, " stdout=", x$stdout, " stderr=", x$stderr, "\n", sep = "")
  if (!is.null(x$stdin_file) || !is.null(x$stdout_file) || !is.null(x$stderr_file)) {
    cat("  files: stdin=", rwasmtime_unset(x$stdin_file),
        " stdout=", rwasmtime_unset(x$stdout_file),
        " stderr=", rwasmtime_unset(x$stderr_file), "\n", sep = "")
  }
  if (length(x$preopens)) {
    preopen_labels <- vapply(x$preopens, function(preopen) {
      paste0(preopen$guest, "=>", preopen$host, if (isTRUE(preopen$readonly)) " (ro)" else " (rw)")
    }, character(1), USE.NAMES = FALSE)
    shown <- utils::head(preopen_labels, 5L)
    cat("  preopens: ", paste(shown, collapse = ", "), if (length(preopen_labels) > length(shown)) ", ..." else "", "\n", sep = "")
  }
  cat("  ambient: network=", x$network, " clocks=", x$clocks, " random=", x$random, "\n", sep = "")
  invisible(x)
}

# Limits ----------------------------------------------------------------------

#' @export
wt_limits <- function() {
  wt_new(
    "WtLimits",
    memory_bytes = NULL,
    table_elements = NULL,
    instances = NULL,
    fuel = NULL,
    wall_time_ms = NULL,
    max_callback_calls = NULL,
    callback_timeout_ms = NULL,
    callback_max_depth = 1L,
    callback_reentrant = FALSE
  )
}

#' @export
wt_limit_memory <- function(.x, bytes) {
  wt_check(.x, "WtLimits")
  .x$memory_bytes <- wt_bytes(bytes)
  .x
}

#' @export
wt_limit_tables <- function(.x, elements) {
  wt_check(.x, "WtLimits")
  .x$table_elements <- rwasmtime_validate_limit_count(elements, "table element limit must be a non-negative whole finite scalar")
  .x
}

#' @export
wt_limit_instances <- function(.x, n) {
  wt_check(.x, "WtLimits")
  .x$instances <- rwasmtime_validate_limit_count(n, "instance limit must be a non-negative whole finite scalar")
  .x
}

#' @export
wt_limit_fuel <- function(.x, fuel) {
  wt_check(.x, "WtLimits")
  .x$fuel <- rwasmtime_validate_limit_count(fuel, "fuel limit must be a non-negative whole finite scalar")
  .x
}

#' @export
wt_limit_wall_time <- function(.x, ms) {
  wt_check(.x, "WtLimits")
  .x$wall_time_ms <- rwasmtime_validate_limit_count(ms, "wall time limit must be a non-negative whole finite scalar")
  .x
}

#' @export
wt_limit_callbacks <- function(.x, max_calls = NULL, timeout_ms = NULL, max_depth = 1L, reentrant = FALSE) {
  wt_check(.x, "WtLimits")
  if (!is.null(max_calls) && (!is.numeric(max_calls) || length(max_calls) != 1L || is.na(max_calls) || !is.finite(max_calls) || max_calls < 0 || max_calls != floor(max_calls))) {
    stop("callback call limit must be NULL or a non-negative whole finite scalar", call. = FALSE)
  }
  if (!is.null(timeout_ms) && (!is.numeric(timeout_ms) || length(timeout_ms) != 1L || is.na(timeout_ms) || !is.finite(timeout_ms) || timeout_ms < 0)) {
    stop("callback timeout must be NULL or a non-negative finite scalar", call. = FALSE)
  }
  if (!is.numeric(max_depth) || length(max_depth) != 1L || is.na(max_depth) || !is.finite(max_depth) || max_depth < 1) {
    stop("callback max_depth must be at least 1", call. = FALSE)
  }
  if (isTRUE(reentrant) && max_depth < 2) {
    stop("reentrant callbacks require max_depth of at least 2", call. = FALSE)
  }
  .x$max_callback_calls <- max_calls
  .x$callback_timeout_ms <- timeout_ms
  .x$callback_max_depth <- as.integer(max_depth)
  .x$callback_reentrant <- isTRUE(reentrant)
  .x
}

#' @export
print.WtLimits <- function(x, ...) {
  cat("<WtLimits> memory=", rwasmtime_unset(x$memory_bytes),
      " tables=", rwasmtime_unset(x$table_elements),
      " instances=", rwasmtime_unset(x$instances), "\n", sep = "")
  cat("  execution: fuel=", rwasmtime_unset(x$fuel),
      " wall_time_ms=", rwasmtime_unset(x$wall_time_ms), "\n", sep = "")
  cat("  callbacks: max_calls=", rwasmtime_unset(x$max_callback_calls),
      " timeout_ms=", rwasmtime_unset(x$callback_timeout_ms),
      " max_depth=", x$callback_max_depth,
      " reentrant=", x$callback_reentrant, "\n", sep = "")
  invisible(x)
}

# Callbacks -------------------------------------------------------------------

#' @export
wt_callback_policy <- function(mode = c("blocking", "fire_and_forget"),
                               thread = c("main"),
                               timeout_ms = NULL,
                               max_calls = NULL,
                               max_depth = 1L,
                               reentrant = FALSE) {
  if (!is.null(timeout_ms) && (!is.numeric(timeout_ms) || length(timeout_ms) != 1L || is.na(timeout_ms) || !is.finite(timeout_ms) || timeout_ms < 0)) {
    stop("callback timeout must be NULL or a non-negative finite scalar", call. = FALSE)
  }
  if (!is.null(max_calls) && (!is.numeric(max_calls) || length(max_calls) != 1L || is.na(max_calls) || !is.finite(max_calls) || max_calls < 0 || max_calls != floor(max_calls))) {
    stop("callback call limit must be NULL or a non-negative whole finite scalar", call. = FALSE)
  }
  if (!is.numeric(max_depth) || length(max_depth) != 1L || is.na(max_depth) || !is.finite(max_depth) || max_depth < 1) {
    stop("callback max_depth must be at least 1", call. = FALSE)
  }
  if (isTRUE(reentrant) && max_depth < 2) {
    stop("reentrant callbacks require max_depth of at least 2", call. = FALSE)
  }
  wt_new(
    "WtCallbackPolicy",
    mode = match.arg(mode),
    thread = match.arg(thread),
    timeout_ms = timeout_ms,
    max_calls = max_calls,
    max_depth = as.integer(max_depth),
    reentrant = isTRUE(reentrant)
  )
}

#' @export
wt_callbacks <- function() {
  wt_new("WtCallbacks", entries = list())
}

#' @export
wt_add_callback <- function(.x,
                            name,
                            fun,
                            params = NULL,
                            results = NULL,
                            module = NULL,
                            abi = c("component", "core", "core_msgpack", "core_memory_request"),
                            policy = wt_callback_policy()) {
  wt_check(.x, "WtCallbacks")
  abi <- match.arg(abi)
  if (!is.function(fun)) stop("fun must be an R function", call. = FALSE)
  if (!inherits(policy, "WtCallbackPolicy")) stop("policy must come from wt_callback_policy()", call. = FALSE)
  if (!is.character(name) || length(name) != 1L || !nzchar(name)) stop("name must be a non-empty string", call. = FALSE)
  if (!is.null(module) && (!is.character(module) || length(module) != 1L || !nzchar(module))) {
    stop("module must be NULL or a non-empty string", call. = FALSE)
  }
  if (identical(abi, "component") && !is.null(module)) {
    stop("component callbacks must not set module", call. = FALSE)
  }
  if (abi %in% c("core", "core_msgpack", "core_memory_request") && is.null(module)) {
    stop("core callbacks require module", call. = FALSE)
  }
  if (identical(policy$mode, "fire_and_forget") && !is.null(results)) {
    stop("fire-and-forget callbacks must not declare results", call. = FALSE)
  }
  key <- if (is.null(module)) name else paste(module, name, sep = "::")
  if (!is.null(.x$entries[[key]])) stop("duplicate callback import: ", key, call. = FALSE)
  .x$entries[[key]] <- list(name = name, module = module, fun = fun, params = params, results = results, abi = abi, policy = policy)
  .x
}

#' @export
print.WtCallbacks <- function(x, ...) {
  cat("<WtCallbacks> entries=", length(x$entries), "\n", sep = "")
  if (length(x$entries)) {
    keys <- names(x$entries)
    shown <- utils::head(keys, 5L)
    cat("  imports: ", paste(shown, collapse = ", "), if (length(keys) > length(shown)) ", ..." else "", "\n", sep = "")
  }
  invisible(x)
}

# App composition -------------------------------------------------------------

#' @export
wt_app <- function(source, kind = c("auto", "module", "component", "artifact")) {
  if (inherits(source, "WtArtifact")) {
    src <- source
    kind <- "artifact"
  } else {
    src <- source
    kind <- match.arg(kind)
  }
  wt_new(
    "WtAppSpec",
    source = src,
    kind = kind,
    runtime = NULL,
    wasi = NULL,
    limits = NULL,
    callbacks = NULL,
    arrays = list(default_dtype = "f64", layout = "column-major", transport = "arena"),
    wit = NULL
  )
}

#' @export
wt_as_module <- function(.x) {
  wt_check(.x, "WtAppSpec")
  .x$kind <- "module"
  .x
}

#' @export
wt_as_component <- function(.x) {
  wt_check(.x, "WtAppSpec")
  .x$kind <- "component"
  .x
}

#' @export
wt_with_runtime <- function(.x, runtime) {
  if (!inherits(.x, "WtAppSpec") && !inherits(.x, "WtComponentSpec")) stop(".x must be an app or component spec", call. = FALSE)
  wt_check(runtime, "WtRuntime", "runtime")
  .x$runtime <- runtime
  .x
}

#' @export
wt_with_wasi <- function(.x, wasi = NULL, ...) {
  if (!inherits(.x, "WtAppSpec") && !inherits(.x, "WtComponentSpec")) stop(".x must be an app or component spec", call. = FALSE)
  dots <- list(...)
  if (is.null(wasi)) wasi <- wt_wasi()
  wt_check(wasi, "WtWasi", "wasi")
  if (length(dots)) {
    allowed <- c("args", "stdin", "stdout", "stderr", "input", "stdin_file", "stdout_file", "stderr_file")
    unknown <- setdiff(names(dots), allowed)
    if (length(unknown)) stop("unsupported wt_with_wasi argument(s): ", paste(unknown, collapse = ", "), call. = FALSE)
    if (!is.null(dots[["args"]])) wasi$args <- as.character(dots[["args"]])
    stdio_names <- c("stdin", "stdout", "stderr", "input", "stdin_file", "stdout_file", "stderr_file")
    if (any(stdio_names %in% names(dots))) {
      wasi <- do.call(wt_wasi_stdio, c(list(.x = wasi), dots[intersect(names(dots), stdio_names)]))
    }
  }
  .x$wasi <- wasi
  .x
}

#' @export
wt_with_limits <- function(.x, limits = NULL, ...) {
  if (!inherits(.x, "WtAppSpec") && !inherits(.x, "WtComponentSpec")) stop(".x must be an app or component spec", call. = FALSE)
  dots <- list(...)
  if (is.null(limits)) limits <- wt_limits()
  wt_check(limits, "WtLimits", "limits")
  if (length(dots)) {
    allowed <- c("memory_bytes", "table_elements", "instances", "wall_time_ms", "fuel")
    dot_names <- names(dots)
    if (is.null(dot_names)) dot_names <- rep("", length(dots))
    unknown <- c(if (any(!nzchar(dot_names))) "<unnamed>", setdiff(dot_names[nzchar(dot_names)], allowed))
    if (length(unknown)) stop("unsupported wt_with_limits argument(s): ", paste(unique(unknown), collapse = ", "), call. = FALSE)
    if (!is.null(dots$memory_bytes)) limits <- wt_limit_memory(limits, dots$memory_bytes)
    if (!is.null(dots$table_elements)) limits <- wt_limit_tables(limits, dots$table_elements)
    if (!is.null(dots$instances)) limits <- wt_limit_instances(limits, dots$instances)
    if (!is.null(dots$wall_time_ms)) limits <- wt_limit_wall_time(limits, dots$wall_time_ms)
    if (!is.null(dots$fuel)) limits <- wt_limit_fuel(limits, dots$fuel)
  }
  .x$limits <- limits
  .x
}

#' @export
wt_with_callbacks <- function(.x, callbacks) {
  if (!inherits(.x, "WtAppSpec") && !inherits(.x, "WtComponentSpec")) stop(".x must be an app or component spec", call. = FALSE)
  wt_check(callbacks, "WtCallbacks", "callbacks")
  .x$callbacks <- callbacks
  .x
}

#' @export
wt_with_arrays <- function(.x,
                           default_dtype = c("f64", "f32", "i32", "i64", "u8"),
                           layout = c("column-major", "row-major", "strided"),
                           transport = c("component", "memory", "arena")) {
  if (!inherits(.x, "WtAppSpec") && !inherits(.x, "WtComponentSpec")) stop(".x must be an app or component spec", call. = FALSE)
  .x$arrays <- list(default_dtype = match.arg(default_dtype), layout = match.arg(layout), transport = match.arg(transport))
  .x
}

#' @export
wt_with_wit <- function(.x, wit, world = NULL, validate = TRUE) {
  if (!inherits(.x, "WtAppSpec") && !inherits(.x, "WtComponentSpec")) stop(".x must be an app or component spec", call. = FALSE)
  .x$wit <- list(wit = wit, world = world, validate = isTRUE(validate))
  .x
}

rwasmtime_same_runtime <- function(a, b) {
  inherits(a, "WtRuntime") &&
    inherits(b, "WtRuntime") &&
    identical(a$backend, b$backend) &&
    identical(a$spec, b$spec) &&
    ((is.null(a$ptr) && is.null(b$ptr)) || identical(a$ptr, b$ptr))
}

rwasmtime_check_same_runtime <- function(a, b, what) {
  if (!rwasmtime_same_runtime(a, b)) {
    stop(what, " must use the same runtime as the artifact", call. = FALSE)
  }
}

rwasmtime_is_native_core_artifact <- function(x) {
  inherits(x, "WtArtifact") && identical(x$backend, "native") && identical(x$kind, "module") && !is.null(x$ptr)
}

rwasmtime_prepare_artifact <- function(spec) {
  runtime <- spec$runtime
  if (!inherits(runtime, "WtRuntime") || !identical(runtime$backend, "native") || is.null(runtime$ptr)) {
    return(NULL)
  }
  if (inherits(spec$source, "WtArtifact")) {
    rwasmtime_check_same_runtime(spec$source$runtime, runtime, "prepared app")
    if (!wt_artifact_compatible(spec$source, runtime)) {
      rwasmtime_stop_aot_incompatible("artifact metadata is incompatible with this runtime", metadata = spec$source$metadata)
    }
    return(spec$source)
  }
  if (inherits(spec, "WtAppSpec") && spec$kind %in% c("auto", "module")) {
    return(wt_compile(runtime, spec$source, kind = "module"))
  }
  NULL
}

#' @export
wt_prepare <- function(.x) {
  if (!inherits(.x, "WtAppSpec") && !inherits(.x, "WtComponentSpec")) stop(".x must be an app or component spec", call. = FALSE)
  if (is.null(.x$runtime)) {
    .x$runtime <- if (inherits(.x$source, "WtArtifact")) .x$source$runtime else wt_build_runtime(wt_runtime_spec())
  }
  artifact <- rwasmtime_prepare_artifact(.x)
  backend <- if (!is.null(artifact) && !is.null(artifact$backend)) artifact$backend else if (inherits(.x$runtime, "WtRuntime") && !is.null(.x$runtime$backend)) .x$runtime$backend else "pending"
  wt_new("WtPreparedApp", spec = .x, artifact = artifact, ptr = NULL, backend = backend)
}

rwasmtime_source_label <- function(source) {
  if (inherits(source, "WtArtifact")) return(paste0("<WtArtifact:", source$kind, ">"))
  as.character(source)[1L]
}

#' @export
print.WtAppSpec <- function(x, ...) {
  cat("<WtAppSpec> kind=", x$kind, " source=", rwasmtime_source_label(x$source), "\n", sep = "")
  invisible(x)
}

#' @export
print.WtPreparedApp <- function(x, ...) {
  cat("<WtPreparedApp> kind=", x$spec$kind, " backend=", if (is.null(x$backend)) "pending" else x$backend,
      " artifact=", inherits(x$artifact, "WtArtifact"), "\n", sep = "")
  invisible(x)
}

# Low-level runtime object stubs ---------------------------------------------

#' @export
wt_compile <- function(.x, input, kind = c("auto", "module", "component")) {
  wt_check(.x, "WtRuntime")
  kind <- match.arg(kind)
  ptr <- NULL
  backend <- "pending"
  if (identical(.x$backend, "native") && !is.null(.x$ptr) && kind %in% c("auto", "module")) {
    ptr <- rwasmtime_compile_core(.x, input)
    backend <- "native"
    kind <- "module"
  }
  metadata <- list(
    format_version = 1L,
    rwasmtime_version = as.character(utils::packageVersion("Rwasmtime")),
    wasmtime_version = if (identical(backend, "native")) "wasmtime-native" else "scaffold",
    target = R.version$platform,
    kind = kind,
    compiler = .x$spec$compiler,
    features = .x$spec$features
  )
  wt_new("WtArtifact", runtime = .x, input = input, kind = kind, ptr = ptr, backend = backend, metadata = metadata)
}

#' @export
wt_aot_save <- function(.x, path, metadata = TRUE, overwrite = FALSE) {
  wt_check(.x, "WtArtifact")
  if (file.exists(path) && !isTRUE(overwrite)) stop("path exists; use overwrite = TRUE", call. = FALSE)
  meta_path <- rwasmtime_aot_metadata_path(path)
  if (file.exists(meta_path) && !isTRUE(overwrite)) stop("metadata sidecar exists; use overwrite = TRUE", call. = FALSE)
  if (identical(.x$backend, "native") && !is.null(.x$ptr) && identical(.x$kind, "module")) {
    bytes <- rwasmtime_serialize_core_module(.x)
    writeBin(bytes, path, useBytes = TRUE)
    if (isTRUE(metadata)) saveRDS(.x$metadata, meta_path)
  }
  .x$aot_path <- path
  .x$aot_metadata <- isTRUE(metadata)
  .x
}

#' @export
wt_aot_load <- function(.x, path, validate = TRUE) {
  wt_check(.x, "WtRuntime")
  metadata <- list(
    format_version = 1L,
    rwasmtime_version = as.character(utils::packageVersion("Rwasmtime")),
    wasmtime_version = "scaffold",
    target = R.version$platform,
    kind = "artifact",
    compiler = .x$spec$compiler,
    features = .x$spec$features,
    validate = isTRUE(validate)
  )
  ptr <- NULL
  backend <- "pending"
  kind <- "artifact"
  if (identical(.x$backend, "native") && !is.null(.x$ptr)) {
    if (!file.exists(path)) stop("AOT artifact does not exist: ", path, call. = FALSE)
    meta_path <- rwasmtime_aot_metadata_path(path)
    if (!file.exists(meta_path)) rwasmtime_stop_aot_incompatible("AOT metadata sidecar is missing", metadata = NULL)
    metadata <- readRDS(meta_path)
    if (isTRUE(validate) && !rwasmtime_aot_compatible(metadata, .x)) {
      rwasmtime_stop_aot_incompatible("AOT artifact metadata is incompatible with this runtime", metadata = metadata)
    }
    size <- file.info(path)$size
    bytes <- readBin(path, what = "raw", n = size)
    ptr <- rwasmtime_deserialize_core(.x, bytes)
    backend <- "native"
    kind <- metadata$kind
    if (is.null(kind) || !identical(kind, "module")) kind <- "module"
  }
  wt_new("WtArtifact", runtime = .x, input = path, kind = kind, ptr = ptr, backend = backend, metadata = metadata)
}

#' @export
wt_artifact_info <- function(.x) {
  wt_check(.x, "WtArtifact")
  wt_new(
    "WtArtifactInfo",
    input = .x$input,
    kind = .x$kind,
    backend = .x$backend,
    metadata = .x$metadata,
    aot_path = if (is.null(.x$aot_path)) NULL else .x$aot_path
  )
}

#' @export
print.WtArtifactInfo <- function(x, ...) {
  compiler <- x$metadata$compiler
  features <- x$metadata$features
  backend <- if (is.null(x$backend)) "pending" else x$backend
  cat("<WtArtifactInfo> kind=", x$kind, " backend=", backend, "\n", sep = "")
  cat("  input: ", as.character(x$input)[1L], "\n", sep = "")
  if (!is.null(x$aot_path)) cat("  aot_path: ", x$aot_path, "\n", sep = "")
  if (!is.null(compiler)) {
    cat("  compiler: ", compiler$strategy, " opt=", compiler$opt_level, "\n", sep = "")
  }
  if (!is.null(features)) {
    shown <- c("component_model", "simd", "relaxed_simd", "memory64", "threads", "exceptions", "legacy_exceptions")
    shown <- shown[shown %in% names(features)]
    values <- paste0(shown, "=", vapply(features[shown], as.character, character(1)))
    cat("  features: ", paste(values, collapse = " "), "\n", sep = "")
  }
  invisible(x)
}

#' @export
wt_artifact_compatible <- function(.x, runtime) {
  wt_check(.x, "WtArtifact")
  wt_check(runtime, "WtRuntime", "runtime")
  metadata <- .x$metadata
  rwasmtime_aot_compatible(metadata, runtime)
}

#' @export
wt_store <- function(.x, limits = NULL, wasi = NULL, callbacks = NULL) {
  wt_check(.x, "WtRuntime")
  if (!is.null(limits)) wt_check(limits, "WtLimits", "limits")
  if (!is.null(wasi)) wt_check(wasi, "WtWasi", "wasi")
  if (!is.null(callbacks)) wt_check(callbacks, "WtCallbacks", "callbacks")
  wt_new("WtStore", runtime = .x, limits = limits, wasi = wasi, callbacks = callbacks, ptr = NULL)
}

#' @export
wt_linker <- function(.x) {
  wt_check(.x, "WtRuntime")
  wt_new("WtLinker", runtime = .x, wasi = NULL, callbacks = NULL, ptr = NULL)
}

#' @export
wt_link_wasi <- function(.x, wasi) {
  wt_check(.x, "WtLinker")
  wt_check(wasi, "WtWasi", "wasi")
  .x$wasi <- wasi
  .x
}

#' @export
wt_link_callbacks <- function(.x, callbacks) {
  wt_check(.x, "WtLinker")
  wt_check(callbacks, "WtCallbacks", "callbacks")
  .x$callbacks <- callbacks
  .x
}

#' @export
wt_instantiate <- function(.x, store, linker) {
  wt_check(.x, "WtArtifact")
  wt_check(store, "WtStore", "store")
  wt_check(linker, "WtLinker", "linker")
  rwasmtime_check_same_runtime(.x$runtime, store$runtime, "store")
  rwasmtime_check_same_runtime(.x$runtime, linker$runtime, "linker")
  ptr <- NULL
  backend <- "pending"
  if (identical(.x$backend, "native") && !is.null(.x$ptr) && identical(.x$kind, "module")) {
    if (!is.null(linker$callbacks) && !is.null(linker$wasi)) {
      ptr <- rwasmtime_instantiate_core_module_wasi_p1_callbacks(.x, linker$wasi, linker$callbacks, store$limits)
    } else if (!is.null(linker$callbacks)) {
      ptr <- rwasmtime_instantiate_core_module_callbacks(.x, linker$callbacks, store$limits)
    } else if (!is.null(linker$wasi)) {
      ptr <- rwasmtime_instantiate_core_module_wasi_p1(.x, linker$wasi, store$limits)
    } else {
      ptr <- rwasmtime_instantiate_core_module(.x, store$limits)
    }
    backend <- "native"
  }
  wt_new("WtInstance", artifact = .x, store = store, linker = linker, ptr = ptr, backend = backend)
}

#' @export
wt_new_session <- function(.x, fresh = TRUE) {
  wt_check(.x, "WtPreparedApp")
  ptr <- NULL
  backend <- "pending"
  runtime <- .x$spec$runtime
  artifact <- .x$artifact
  is_native_module <- rwasmtime_is_native_core_artifact(artifact)
  if (inherits(runtime, "WtRuntime") && identical(runtime$backend, "native") && !is.null(runtime$ptr) &&
      is_native_module && !is.null(.x$spec$wasi) && !is.null(.x$spec$callbacks)) {
    wt_not_implemented("wt_new_session for native WASI apps with callbacks")
  }
  if (inherits(runtime, "WtRuntime") && identical(runtime$backend, "native") && !is.null(runtime$ptr) &&
      is_native_module && is.null(.x$spec$wasi)) {
    ptr <- if (!is.null(.x$spec$callbacks)) {
      rwasmtime_instantiate_core_module_callbacks(artifact, .x$spec$callbacks, .x$spec$limits)
    } else {
      rwasmtime_instantiate_core_module(artifact, .x$spec$limits)
    }
    backend <- "native"
  }
  wt_new("WtSession", app = .x, fresh = isTRUE(fresh), ptr = ptr, backend = backend, temp_arrays = list())
}

#' @export
wt_exec <- function(.x, export, ..., .args = NULL) {
  args <- if (is.null(.args)) list(...) else .args
  if (inherits(.x, "WtSession") || inherits(.x, "WtPreparedApp") || inherits(.x, "WtInstance")) {
    invisible(wt_call(.x, export, .args = args))
    return(.x)
  }
  wt_not_implemented(paste0("wt_exec(", export, ")"))
}

#' @export
wt_call <- function(.x, export, ..., .args = NULL) {
  args <- if (is.null(.args)) list(...) else .args
  if (inherits(.x, "WtSession")) {
    if (identical(.x$backend, "native") && !is.null(.x$ptr)) {
      if (rwasmtime_is_native_core_artifact(.x$app$artifact) && is.null(.x$app$spec$wasi)) {
        if (!is.null(.x$app$spec$callbacks)) {
          return(rwasmtime_with_callback_errors(rwasmtime_call_instance_core(.x, export, args)))
        }
        return(rwasmtime_with_wasm_call_errors(rwasmtime_call_instance_core(.x, export, args)))
      }
      wt_not_implemented(paste0("wt_call(", export, ") for non-core module session"))
    }
  }
  if (inherits(.x, "WtInstance")) {
    if (identical(.x$backend, "native") && !is.null(.x$ptr)) {
      if (identical(.x$artifact$kind, "module")) {
        result <- if (!is.null(.x$linker$callbacks)) {
          rwasmtime_with_callback_errors(rwasmtime_call_instance_core(.x, export, args))
        } else {
          rwasmtime_with_wasm_call_errors(rwasmtime_call_instance_core(.x, export, args))
        }
        if (!is.null(.x$linker$wasi) && identical(export, "_start") && !length(args)) {
          return(rwasmtime_instance_wasi_output(.x, wasi = .x$linker$wasi))
        }
        return(result)
      }
      wt_not_implemented(paste0("wt_call(", export, ") for non-core instance"))
    }
  }
  if (inherits(.x, "WtPreparedApp")) {
    runtime <- .x$spec$runtime
    artifact <- .x$artifact
    is_native_module <- rwasmtime_is_native_core_artifact(artifact)
    if (inherits(runtime, "WtRuntime") && identical(runtime$backend, "native") && !is.null(runtime$ptr)) {
      if (is_native_module) {
        if (!is.null(.x$spec$wasi)) {
          if (!is.null(.x$spec$callbacks)) {
            wt_not_implemented("wt_call for native WASI apps with callbacks")
          }
          if (!identical(export, "_start")) {
            wt_not_implemented(paste0("wt_call(", export, ") for WASI command apps"))
          }
          if (length(args)) stop("WASI command _start does not accept R arguments", call. = FALSE)
          return(rwasmtime_with_wasm_call_errors(rwasmtime_call_core_module_wasi_p1(artifact, .x$spec$wasi, .x$spec$limits)))
        }
        if (!is.null(.x$spec$callbacks)) {
          return(rwasmtime_with_callback_errors(rwasmtime_call_core_module_callbacks(artifact, export, args, .x$spec$callbacks, .x$spec$limits)))
        }
        return(rwasmtime_with_wasm_call_errors(rwasmtime_call_core_module(artifact, export, args, .x$spec$limits)))
      }
      wt_not_implemented(paste0("wt_call(", export, ") for non-core module app"))
    }
  }
  wt_not_implemented(paste0("wt_call(", export, ")"))
}

rwasmtime_can_call_immediately <- function(.x) {
  if (inherits(.x, "WtSession")) {
    return(identical(.x$backend, "native") && !is.null(.x$ptr) && rwasmtime_is_native_core_artifact(.x$app$artifact) && is.null(.x$app$spec$wasi))
  }
  if (inherits(.x, "WtInstance")) {
    return(identical(.x$backend, "native") && !is.null(.x$ptr) && identical(.x$artifact$kind, "module"))
  }
  if (inherits(.x, "WtPreparedApp")) {
    runtime <- .x$spec$runtime
    return(inherits(runtime, "WtRuntime") && identical(runtime$backend, "native") && !is.null(runtime$ptr) && rwasmtime_is_native_core_artifact(.x$artifact))
  }
  FALSE
}

#' @export
wt_call_async <- function(.x, export, ..., .args = NULL) {
  args <- if (is.null(.args)) list(...) else .args
  job <- wt_new_env("WtJob", app = .x, export = export, args = args, state = "pending", result = NULL, error = NULL)
  if (rwasmtime_can_call_immediately(.x)) {
    result <- tryCatch(wt_call(.x, export, .args = args), error = identity)
    if (inherits(result, "error")) {
      job$state <- "error"
      job$error <- result
    } else {
      job$state <- "done"
      job$result <- result
    }
  }
  job
}

#' @export
wt_poll <- function(.x) {
  wt_check(.x, "WtJob")
  wt_new(
    "WtJobStatus",
    done = identical(.x$state, "done"),
    cancelled = identical(.x$state, "cancelled"),
    error = identical(.x$state, "error"),
    state = .x$state,
    export = .x$export
  )
}

#' @export
wt_await <- function(.x, timeout_ms = NULL) {
  wt_check(.x, "WtJob")
  timeout_ms <- rwasmtime_validate_timeout_ms(timeout_ms)
  if (identical(.x$state, "done")) return(.x$result)
  if (identical(.x$state, "error")) stop(.x$error)
  if (identical(.x$state, "cancelled")) stop("job was cancelled", call. = FALSE)
  if (!is.null(timeout_ms)) {
    if (timeout_ms > 0) Sys.sleep(timeout_ms / 1000)
    if (identical(.x$state, "done")) return(.x$result)
    if (identical(.x$state, "error")) stop(.x$error)
    if (identical(.x$state, "cancelled")) stop("job was cancelled", call. = FALSE)
    rwasmtime_stop_timeout("job did not complete before timeout_ms", timeout_ms = timeout_ms, job = .x)
  }
  wt_not_implemented("wt_await")
}

#' @export
wt_drain_callbacks <- function(.x, max = Inf) {
  wt_check(.x, "WtJob")
  .x
}

#' @export
wt_result <- function(.x) {
  wt_check(.x, "WtJob")
  if (!identical(.x$state, "done")) return(NULL)
  .x$result
}

#' @export
wt_cancel <- function(.x) {
  wt_check(.x, "WtJob")
  .x$state <- "cancelled"
  .x
}

#' @export
print.WtJob <- function(x, ...) {
  cat("<WtJob> export=", x$export, " state=", x$state, "\n", sep = "")
  invisible(x)
}

#' @export
print.WtJobStatus <- function(x, ...) {
  cat("<WtJobStatus> export=", x$export, " state=", x$state,
      " done=", x$done, " cancelled=", x$cancelled,
      " error=", x$error, "\n", sep = "")
  invisible(x)
}

#' @export
wt_imports <- function(.x) {
  if (rwasmtime_is_component_like(.x)) {
    return(wt_component_imports(.x))
  }
  rwasmtime_native_core_items(.x, "imports")
}

#' @export
wt_exports <- function(.x) {
  if (rwasmtime_is_component_like(.x)) {
    return(wt_component_exports(.x))
  }
  rwasmtime_native_core_items(.x, "exports")
}

#' @export
wt_bindings <- function(.x) {
  wt_new("WtBindings", imports = wt_imports(.x), exports = wt_exports(.x))
}

#' @export
print.WtArtifact <- function(x, ...) {
  cat("<WtArtifact> kind=", x$kind, " backend=", if (is.null(x$backend)) "pending" else x$backend, "\n", sep = "")
  cat("  input: ", as.character(x$input)[1L], "\n", sep = "")
  if (!is.null(x$aot_path)) cat("  aot_path: ", x$aot_path, "\n", sep = "")
  invisible(x)
}

#' @export
print.WtCoreItem <- function(x, ...) {
  qualifier <- if (!is.null(x$module)) paste0(x$module, "::") else ""
  cat("<WtCoreItem> ", x$direction, " ", qualifier, x$name, " kind=", x$kind, sep = "")
  if (!is.null(x$signature) && nzchar(x$signature)) cat(" ", x$signature, sep = "")
  cat("\n")
  invisible(x)
}

#' @export
print.WtBindings <- function(x, ...) {
  cat("<WtBindings> imports=", length(x$imports), " exports=", length(x$exports), "\n", sep = "")
  invisible(x)
}

#' @export
print.WtStore <- function(x, ...) {
  cat("<WtStore> backend=", if (is.null(x$runtime$backend)) "pending" else x$runtime$backend,
      " limits=", !is.null(x$limits), " wasi=", !is.null(x$wasi),
      " callbacks=", !is.null(x$callbacks), "\n", sep = "")
  invisible(x)
}

#' @export
print.WtLinker <- function(x, ...) {
  cat("<WtLinker> backend=", if (is.null(x$runtime$backend)) "pending" else x$runtime$backend,
      " wasi=", !is.null(x$wasi), " callbacks=", !is.null(x$callbacks), "\n", sep = "")
  invisible(x)
}

#' @export
print.WtInstance <- function(x, ...) {
  cat("<WtInstance> kind=", x$artifact$kind, " backend=", if (is.null(x$backend)) "pending" else x$backend, "\n", sep = "")
  invisible(x)
}

#' @export
print.WtSession <- function(x, ...) {
  cat("<WtSession> kind=", x$app$spec$kind, " backend=", x$backend, " fresh=", x$fresh, "\n", sep = "")
  invisible(x)
}

# Memory and arrays -----------------------------------------------------------

#' @export
wt_memory <- function(.x, name = "memory") {
  if (!is.character(name) || length(name) != 1L || !nzchar(name)) stop("memory name must be a non-empty string", call. = FALSE)
  wt_new("WtMemory", owner = .x, name = name, ptr = NULL)
}

#' @export
print.WtMemory <- function(x, ...) {
  owner_backend <- if (!is.null(x$owner$backend)) x$owner$backend else "pending"
  cat("<WtMemory> name=", x$name, " owner_backend=", owner_backend, "\n", sep = "")
  invisible(x)
}

#' @export
wt_memory_size <- function(.x) {
  wt_check(.x, "WtMemory")
  if (!is.null(rwasmtime_memory_instance(.x))) return(rwasmtime_memory_size(.x))
  wt_not_implemented("wt_memory_size")
}

#' @export
wt_memory_grow <- function(.x, pages) {
  wt_check(.x, "WtMemory")
  if (!is.numeric(pages) || length(pages) != 1L || is.na(pages) || !is.finite(pages) || pages < 0 || pages != floor(pages)) {
    stop("pages must be a non-negative whole number", call. = FALSE)
  }
  if (!is.null(rwasmtime_memory_instance(.x))) return(rwasmtime_memory_grow(.x, pages))
  wt_not_implemented("wt_memory_grow")
}

#' @export
wt_memory_read <- function(.x,
                           ptr,
                           length,
                           dtype = c("u8", "i32", "u32", "i64", "u64", "f32", "f64", "v128"),
                           dim = NULL,
                           layout = c("contiguous", "row-major", "column-major")) {
  wt_check(.x, "WtMemory")
  dtype <- match.arg(dtype)
  layout <- match.arg(layout)
  if (identical(dtype, "v128")) wt_not_implemented("wt_memory_read dtype=v128")
  ptr <- rwasmtime_scalar_whole_nonnegative(ptr, "ptr")
  length <- rwasmtime_scalar_whole_nonnegative(length, "length")
  byte_length <- length * rwasmtime_dtype_size(dtype)
  if (!is.null(rwasmtime_memory_instance(.x))) {
    out <- rwasmtime_unpack_memory(rwasmtime_memory_read_raw(.x, ptr, byte_length), dtype, length)
    if (!is.null(dim)) {
      if (prod(dim) != base::length(out)) stop("dim product must match typed memory read length", call. = FALSE)
      dim(out) <- dim
    }
    return(out)
  }
  wt_not_implemented("wt_memory_read")
}

#' @export
wt_memory_write <- function(.x,
                            ptr,
                            value,
                            dtype = NULL,
                            layout = c("contiguous", "row-major", "column-major")) {
  wt_check(.x, "WtMemory")
  layout <- match.arg(layout)
  if (is.null(dtype)) dtype <- if (is.raw(value)) "u8" else "f64"
  dtype <- match.arg(dtype, c("u8", "i32", "u32", "i64", "u64", "f32", "f64", "v128"))
  ptr <- rwasmtime_scalar_whole_nonnegative(ptr, "ptr")
  if (!is.null(rwasmtime_memory_instance(.x))) {
    rwasmtime_memory_write_raw(.x, ptr, rwasmtime_pack_memory(value, dtype))
    return(.x)
  }
  wt_not_implemented("wt_memory_write")
}

#' @export
wt_memory_view <- function(.x, ptr, length, dtype, mutable = FALSE, lifetime = "until_next_wasm_call") {
  wt_check(.x, "WtMemory")
  wt_new("WtMemoryView", memory = .x, ptr = ptr, length = length, dtype = dtype, mutable = isTRUE(mutable), lifetime = lifetime)
}

#' @export
wt_array_write <- function(.x,
                           value,
                           dtype = c("f64", "f32", "i32", "u32", "i64", "u64", "u8"),
                           layout = c("column-major", "row-major", "contiguous"),
                           allocator = c("guest", "host_arena"),
                           alloc_export = "alloc",
                           free_export = "free",
                           memory = "memory") {
  dtype <- match.arg(dtype)
  layout <- match.arg(layout)
  allocator <- match.arg(allocator)
  if (!inherits(.x, "WtSession") && !inherits(.x, "WtInstance")) wt_not_implemented("wt_array_write")
  if (!identical(allocator, "guest")) wt_not_implemented("wt_array_write allocator=host_arena")
  if (!identical(.x$backend, "native") || is.null(.x$ptr)) wt_not_implemented("wt_array_write")
  memory <- rwasmtime_scalar_string(memory, "memory")
  values <- rwasmtime_array_values(value, layout)
  payload <- rwasmtime_pack_memory(values, dtype)
  byte_length <- length(payload)
  mem <- .x |> wt_memory(memory)
  rwasmtime_check_memory_object_limit(mem, byte_length, "wt_array_write")
  ptr <- rwasmtime_scalar_whole_nonnegative(wt_call(.x, alloc_export, byte_length), "guest allocation pointer")
  rwasmtime_check_memory_object_limit(mem, ptr + byte_length, "wt_array_write")
  mem |> wt_memory_write(ptr = ptr, value = payload, dtype = "u8")
  wt_new(
    "WtArray",
    owner = .x,
    memory = memory,
    ptr = ptr,
    length = rwasmtime_array_length(values, dtype),
    byte_length = byte_length,
    dtype = dtype,
    dim = dim(value),
    layout = layout,
    free_export = free_export,
    freed = FALSE
  )
}

#' @export
print.WtArray <- function(x, ...) {
  cat("<WtArray> dtype=", x$dtype, " length=", x$length,
      " bytes=", x$byte_length, " ptr=", x$ptr,
      " freed=", isTRUE(x$freed), "\n", sep = "")
  invisible(x)
}

#' @export
print.WtMemoryView <- function(x, ...) {
  cat("<WtMemoryView> dtype=", x$dtype, " ptr=", x$ptr,
      " length=", x$length, " mutable=", x$mutable,
      " lifetime=", x$lifetime, "\n", sep = "")
  invisible(x)
}

#' @export
wt_as_array <- function(.x, dtype, dim = NULL, layout = c("column-major", "row-major", "contiguous")) {
  layout <- match.arg(layout)
  if (inherits(.x, "WtArray")) {
    if (missing(dtype) || is.null(dtype)) dtype <- .x$dtype
    dtype <- match.arg(dtype, c("u8", "i32", "u32", "i64", "u64", "f32", "f64", "v128"))
    if (is.null(dim)) dim <- .x$dim
    out <- .x$owner |>
      wt_memory(.x$memory) |>
      wt_memory_read(ptr = .x$ptr, length = .x$length, dtype = dtype)
    return(rwasmtime_shape_array(out, dim, if (missing(layout)) .x$layout else layout))
  }
  wt_not_implemented("wt_as_array")
}

#' @export
wt_with_temp_array <- function(.x,
                               name,
                               value,
                               dtype = NULL,
                               layout = c("column-major", "row-major", "contiguous")) {
  .x$temp_arrays[[name]] <- list(value = value, dtype = dtype, layout = match.arg(layout))
  .x
}

#' @export
wt_arg_array <- function(name) {
  wt_new("WtArrayArgument", name = name)
}

#' @export
print.WtArrayArgument <- function(x, ...) {
  cat("<WtArrayArgument> name=", x$name, "\n", sep = "")
  invisible(x)
}

#' @export
wt_free <- function(.x) {
  if (inherits(.x, "WtRepl")) return(wt_repl_close(.x))
  if (inherits(.x, "WtArray")) {
    if (!isTRUE(.x$freed) && !is.null(.x$free_export) && nzchar(.x$free_export)) {
      invisible(wt_call(.x$owner, .x$free_export, .x$ptr, .x$byte_length))
    }
    .x$freed <- TRUE
    return(invisible(.x))
  }
  wt_not_implemented("wt_free")
}

# Components ------------------------------------------------------------------

rwasmtime_component_spec <- function(.x) {
  if (inherits(.x, "WtPreparedApp")) return(.x$spec)
  .x
}

rwasmtime_component_items <- function(items) {
  lapply(items, function(item) {
    wt_new(
      "WtComponentItem",
      name = item$name,
      kind = item$kind,
      interface = item$interface,
      params_schema = item$params_schema,
      results_schema = item$results_schema
    )
  })
}

rwasmtime_core_items <- function(items, direction = c("imports", "exports")) {
  direction <- match.arg(direction)
  lapply(items, function(item) {
    wt_new(
      "WtCoreItem",
      direction = sub("s$", "", direction),
      module = item$module,
      name = item$name,
      kind = item$kind,
      params = item$params,
      results = item$results,
      minimum = item$minimum,
      maximum = item$maximum,
      shared = item$shared,
      memory64 = item$memory64,
      mutable = item$mutable,
      element = item$element,
      value_type = item$value_type,
      signature = item$signature
    )
  })
}

rwasmtime_is_component_like <- function(.x) {
  inherits(.x, "WtComponentSpec") ||
    (inherits(.x, "WtAppSpec") && identical(.x$kind, "component")) ||
    (inherits(.x, "WtPreparedApp") && (
      inherits(.x$spec, "WtComponentSpec") ||
        (inherits(.x$spec, "WtAppSpec") && identical(.x$spec$kind, "component"))
    ))
}

rwasmtime_native_core_items <- function(.x, direction = c("imports", "exports")) {
  direction <- match.arg(direction)
  artifact <- NULL
  if (rwasmtime_is_native_core_artifact(.x)) {
    artifact <- .x
  } else if (inherits(.x, "WtPreparedApp") && rwasmtime_is_native_core_artifact(.x$artifact)) {
    artifact <- .x$artifact
  } else if (inherits(.x, "WtAppSpec") && .x$kind %in% c("auto", "module") && inherits(.x$runtime, "WtRuntime")) {
    artifact <- rwasmtime_prepare_artifact(.x)
  }
  if (!rwasmtime_is_native_core_artifact(artifact)) {
    wt_not_implemented(paste0("wt_", direction, " for core modules"))
  }
  items <- if (identical(direction, "imports")) artifact$ptr$imports() else artifact$ptr$exports()
  rwasmtime_core_items(items, direction)
}

rwasmtime_native_component_items <- function(.x, direction = c("exports", "imports")) {
  direction <- match.arg(direction)
  spec <- rwasmtime_component_spec(.x)
  if (!inherits(spec, "WtComponentSpec") && !(inherits(spec, "WtAppSpec") && identical(spec$kind, "component"))) {
    wt_not_implemented(paste0("wt_component_", direction))
  }
  runtime <- spec$runtime
  if (inherits(runtime, "WtRuntime") && identical(runtime$backend, "native") && !is.null(runtime$ptr) && !inherits(spec$source, "WtArtifact")) {
    items <- if (identical(direction, "exports")) {
      rwasmtime_with_compile_errors(runtime$ptr$component_exports(as.character(spec$source)))
    } else {
      rwasmtime_with_compile_errors(runtime$ptr$component_imports(as.character(spec$source)))
    }
    return(rwasmtime_component_items(items))
  }
  wt_not_implemented(paste0("wt_component_", direction))
}

#' @export
wt_component <- function(source) {
  wt_new(
    "WtComponentSpec",
    source = source,
    runtime = NULL,
    wasi = NULL,
    limits = NULL,
    callbacks = NULL,
    arrays = list(default_dtype = "f64", layout = "column-major", transport = "arena"),
    wit = NULL
  )
}

#' @export
wt_component_exports <- function(.x) {
  rwasmtime_native_component_items(.x, "exports")
}

#' @export
wt_component_imports <- function(.x) {
  rwasmtime_native_component_items(.x, "imports")
}

#' @export
print.WtComponentSpec <- function(x, ...) {
  cat("<WtComponentSpec> source=", rwasmtime_source_label(x$source),
      " wit=", !is.null(x$wit), "\n", sep = "")
  invisible(x)
}

#' @export
print.WtComponentItem <- function(x, ...) {
  cat("<WtComponentItem> name=", x$name, " kind=", x$kind, sep = "")
  if (!is.null(x$params_schema)) cat(" params=", x$params_schema, sep = "")
  if (!is.null(x$results_schema)) cat(" results=", x$results_schema, sep = "")
  cat("\n")
  invisible(x)
}

# REPL protocol ---------------------------------------------------------------

#' @export
wt_repl <- function(.x = NULL,
                    protocol = c("component", "stdio", "callback", "core", "mock"),
                    eval_export = "eval",
                    prompt = "> ",
                    continuation = "+ ",
                    guest = NULL,
                    ...) {
  protocol <- match.arg(protocol)
  dots <- list(...)
  if (!is.character(prompt) || length(prompt) != 1L || !nzchar(prompt)) stop("prompt must be a non-empty string", call. = FALSE)
  if (!is.character(continuation) || length(continuation) != 1L || !nzchar(continuation)) stop("continuation must be a non-empty string", call. = FALSE)
  if (protocol %in% c("component", "callback", "core") && (!is.character(eval_export) || length(eval_export) != 1L || !nzchar(eval_export))) {
    stop("component/callback/core REPL protocols require eval_export", call. = FALSE)
  }
  if (identical(protocol, "mock") && !is.null(guest) && !identical(guest, "mock")) {
    stop("mock REPL protocol is reserved for scaffold tests", call. = FALSE)
  }
  if (identical(protocol, "mock")) guest <- "mock"
  session <- if (identical(protocol, "core")) rwasmtime_core_repl_session(.x) else NULL
  wt_new_env(
    "WtRepl",
    app = .x,
    session = session,
    protocol = protocol,
    protocol_options = if (identical(protocol, "core")) rwasmtime_core_repl_options(dots) else list(),
    eval_export = eval_export,
    prompt = prompt,
    continuation = continuation,
    guest = guest,
    open = TRUE,
    input = character(),
    output = character(),
    results = list()
  )
}

#' @export
wt_webr_repl <- function(source,
                         runtime = NULL,
                         wasi = NULL,
                         limits = NULL,
                         callbacks = NULL,
                         protocol = c("component", "stdio"),
                         eval_export = "webr:host/repl.eval") {
  protocol <- match.arg(protocol)
  if (identical(protocol, "component") && (!is.character(eval_export) || length(eval_export) != 1L || !nzchar(eval_export))) {
    stop("webR component REPL requires eval_export", call. = FALSE)
  }
  app <- if (inherits(source, "WtPreparedApp")) source else wt_as_component(wt_app(source))
  if (!is.null(runtime) && inherits(app, "WtAppSpec")) app <- wt_with_runtime(app, runtime)
  if (!is.null(wasi) && inherits(app, "WtAppSpec")) app <- wt_with_wasi(app, wasi)
  if (!is.null(limits) && inherits(app, "WtAppSpec")) app <- wt_with_limits(app, limits)
  if (!is.null(callbacks) && inherits(app, "WtAppSpec")) app <- wt_with_callbacks(app, callbacks)
  prepared <- if (inherits(app, "WtPreparedApp")) app else wt_prepare(app)
  wt_repl(prepared, protocol = protocol, eval_export = eval_export, guest = "webR")
}

#' @export
wt_repl_send <- function(.x, code) {
  wt_check(.x, "WtRepl")
  if (!isTRUE(.x$open)) stop("REPL is closed", call. = FALSE)
  if (identical(.x$protocol, "core")) {
    result <- rwasmtime_core_repl_eval(.x, code)
    .x$input <- c(.x$input, result$input)
    .x$output <- c(.x$output, result$value)
    .x$results[[length(.x$results) + 1L]] <- result
    return(.x)
  }
  if (!identical(.x$protocol, "mock")) {
    wt_not_implemented(paste0("wt_repl_send protocol=", .x$protocol))
  }
  .x$input <- c(.x$input, as.character(code))
  out <- paste0("<mock sandbox> ", as.character(code))
  .x$output <- c(.x$output, out)
  .x$results[[length(.x$results) + 1L]] <- wt_new("WtReplResult", input = as.character(code), stdout = out, stderr = character(), value = NULL, error = NULL, status = 0L, complete = TRUE)
  .x
}

#' @export
wt_repl_read <- function(.x, n = Inf) {
  wt_check(.x, "WtRepl")
  out <- .x$output
  if (is.finite(n)) out <- utils::tail(out, as.integer(n))
  out
}

#' @export
wt_repl_eval <- function(.x, code) {
  wt_check(.x, "WtRepl")
  if (.x$protocol %in% c("mock", "core")) {
    .x <- wt_repl_send(.x, code)
    return(.x$results[[length(.x$results)]])
  }
  wt_not_implemented(paste0("wt_repl_eval protocol=", .x$protocol))
}

#' @export
wt_repl_history <- function(.x) {
  wt_check(.x, "WtRepl")
  .x$input
}

#' @export
wt_repl_info <- function(.x) {
  wt_check(.x, "WtRepl")
  wt_new(
    "WtReplInfo",
    protocol = .x$protocol,
    eval_export = .x$eval_export,
    guest = .x$guest,
    open = .x$open,
    inputs = length(.x$input),
    backend = if (inherits(.x$session, "WtSession")) .x$session$backend else "pending",
    protocol_options = .x$protocol_options
  )
}

#' @export
wt_repl_close <- function(.x) {
  wt_check(.x, "WtRepl")
  .x$open <- FALSE
  .x
}

#' @export
print.WtRepl <- function(x, ...) {
  cat("<WtRepl> protocol=", x$protocol, " guest=", if (is.null(x$guest)) "generic" else x$guest,
      " open=", x$open, "\n", sep = "")
  invisible(x)
}

#' @export
print.WtReplInfo <- function(x, ...) {
  cat("<WtReplInfo> protocol=", x$protocol,
      " guest=", if (is.null(x$guest)) "generic" else x$guest,
      " backend=", x$backend, " open=", x$open,
      " inputs=", x$inputs, "\n", sep = "")
  if (!is.null(x$eval_export)) cat("  eval_export: ", x$eval_export, "\n", sep = "")
  invisible(x)
}

#' @export
print.WtReplResult <- function(x, ...) {
  has_error <- !is.null(x$error) && length(x$error) == 1L && nzchar(x$error)
  cat("<WtReplResult> complete=", x$complete, " status=", if (is.null(x$status)) "NA" else x$status,
      " stdout_lines=", length(x$stdout), " error=", has_error, "\n", sep = "")
  invisible(x)
}

#' @export
print.WtWasiResult <- function(x, ...) {
  cat("<WtWasiResult> stdout_bytes=", length(x$stdout_raw),
      " stderr_bytes=", length(x$stderr_raw), "\n", sep = "")
  if (!is.null(x$stdout_file)) cat("  stdout_file: ", x$stdout_file, "\n", sep = "")
  if (!is.null(x$stderr_file)) cat("  stderr_file: ", x$stderr_file, "\n", sep = "")
  if (length(x$stdout_raw) && !is.na(x$stdout)) cat("  stdout: ", substr(x$stdout, 1L, 80L), "\n", sep = "")
  if (length(x$stdout_raw) && is.na(x$stdout)) cat("  stdout: <non-text bytes>\n", sep = "")
  if (length(x$stderr_raw) && !is.na(x$stderr)) cat("  stderr: ", substr(x$stderr, 1L, 80L), "\n", sep = "")
  if (length(x$stderr_raw) && is.na(x$stderr)) cat("  stderr: <non-text bytes>\n", sep = "")
  invisible(x)
}

#' @export
print.WtCallbackPolicy <- function(x, ...) {
  cat("<WtCallbackPolicy> mode=", x$mode, " thread=", x$thread,
      " timeout_ms=", rwasmtime_unset(x$timeout_ms),
      " max_calls=", rwasmtime_unset(x$max_calls),
      " max_depth=", x$max_depth,
      " reentrant=", x$reentrant, "\n", sep = "")
  invisible(x)
}
