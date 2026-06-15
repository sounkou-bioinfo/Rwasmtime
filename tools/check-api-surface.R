#!/usr/bin/env Rscript

api_path <- file.path("R", "api.R")
namespace_path <- "NAMESPACE"
rd_path <- file.path("man", "Rwasmtime-api.Rd")
for (path in c(api_path, namespace_path, rd_path)) {
  if (!file.exists(path)) stop("missing required file: ", path, call. = FALSE)
}

preferred_public <- c(
  # Runtime
  "wt_runtime_spec", "wt_with_compiler", "wt_enable_features",
  "wt_with_aot", "wt_with_allocator", "wt_build_runtime",
  # WASI
  "wt_wasi", "wt_wasi_args", "wt_wasi_env", "wt_wasi_preopen",
  "wt_wasi_stdio", "wt_wasi_network",
  # Limits
  "wt_limits", "wt_limit_memory", "wt_limit_tables",
  "wt_limit_instances", "wt_limit_fuel", "wt_limit_wall_time",
  "wt_limit_callbacks",
  # Callbacks
  "wt_callbacks", "wt_callback_policy", "wt_add_callback",
  # App
  "wt_app", "wt_as_module", "wt_as_component", "wt_with_runtime",
  "wt_with_wasi", "wt_with_limits", "wt_with_callbacks",
  "wt_with_arrays", "wt_with_wit", "wt_prepare",
  # Execution
  "wt_call", "wt_exec", "wt_call_async", "wt_poll",
  "wt_await", "wt_drain_callbacks", "wt_result", "wt_cancel",
  # AOT
  "wt_compile", "wt_aot_save", "wt_aot_load", "wt_artifact_info",
  "wt_artifact_compatible",
  # Memory/arrays
  "wt_memory", "wt_memory_size", "wt_memory_grow",
  "wt_memory_read", "wt_memory_write", "wt_memory_view",
  "wt_array_write", "wt_as_array", "wt_with_temp_array", "wt_arg_array",
  "wt_free",
  # REPL
  "wt_repl", "wt_webr_repl", "wt_repl_send", "wt_repl_read",
  "wt_repl_eval", "wt_repl_history", "wt_repl_info", "wt_repl_close"
)

scaffold_public <- c(
  "wt_new_session",
  "wt_store", "wt_linker", "wt_link_wasi", "wt_link_callbacks",
  "wt_instantiate", "wt_component", "wt_component_exports",
  "wt_component_imports"
)

internal_helpers <- c(
  "wt_bytes", "wt_check", "wt_choose", "wt_new", "wt_new_env",
  "wt_not_implemented", "wt_set_non_null"
)

if (file.exists("AGENTS.md")) {
  agents <- paste(readLines("AGENTS.md", warn = FALSE), collapse = "\n")
  block <- sub("(?s)^.*Preferred public functions:", "", agents, perl = TRUE)
  block <- sub("(?s)## Build/development workflow.*$", "", block, perl = TRUE)
  from_agents <- unique(unlist(regmatches(block, gregexpr("`(wt_[A-Za-z0-9_]+)\\(", block, perl = TRUE))))
  from_agents <- sub("^`", "", sub("\\($", "", from_agents))
  if (length(from_agents)) {
    missing_from_gate <- setdiff(from_agents, preferred_public)
    stale_in_gate <- setdiff(preferred_public, from_agents)
    if (length(missing_from_gate) || length(stale_in_gate)) {
      stop(sprintf(
        "API-surface gate is out of sync with AGENTS.md preferred functions\nmissing from gate: %s\nstale in gate: %s",
        paste(sort(missing_from_gate), collapse = ", "),
        paste(sort(stale_in_gate), collapse = ", ")
      ), call. = FALSE)
    }
  }
}

expected_public <- unique(c(preferred_public, scaffold_public))
expected_defined <- unique(c(expected_public, internal_helpers))

api <- readLines(api_path, warn = FALSE)
namespace <- readLines(namespace_path, warn = FALSE)
rd <- readLines(rd_path, warn = FALSE)

defined <- unique(sub(
  "^([A-Za-z0-9_]+)\\s*<-\\s*function\\s*\\(.*$",
  "\\1",
  grep("^[A-Za-z0-9_]+\\s*<-\\s*function\\s*\\(", api, value = TRUE)
))
defined_wt <- sort(grep("^wt_", defined, value = TRUE))

exported <- unique(sub("^export\\(([^)]+)\\)$", "\\1", grep("^export\\(", namespace, value = TRUE)))
exported_wt <- sort(grep("^wt_", exported, value = TRUE))

aliases <- unique(sub("^\\\\alias\\{([^}]+)\\}.*$", "\\1", grep("^\\\\alias\\{", rd, value = TRUE)))
aliases_wt <- sort(grep("^wt_", aliases, value = TRUE))

fail <- function(label, values) {
  if (length(values)) {
    stop(sprintf("%s:\n%s", label, paste(sprintf("  - %s", sort(values)), collapse = "\n")), call. = FALSE)
  }
}

fail("expected public wt_* functions are not defined", setdiff(expected_public, defined_wt))
fail("expected public wt_* functions are not exported", setdiff(expected_public, exported_wt))
fail("expected public wt_* functions are not documented as aliases", setdiff(expected_public, aliases_wt))
fail("internal helper wt_* functions were exported", intersect(internal_helpers, exported_wt))
fail("unexpected exported wt_* functions", setdiff(exported_wt, expected_public))
fail("unexpected defined wt_* functions; classify as public or helper", setdiff(defined_wt, expected_defined))

cat(sprintf(
  "API surface check ok: %d preferred public, %d scaffold public, %d internal helpers\n",
  length(preferred_public), length(scaffold_public), length(internal_helpers)
))
