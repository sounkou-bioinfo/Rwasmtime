# Rwasmtime (development version)

## Site and CI

- Expanded `_pkgdown.yml` into a Ropendal-style site map covering runtime
  construction, capabilities, app execution, low-level artifacts, memory/arrays,
  component metadata, and REPL protocols, plus the two package vignettes as
  articles. Local `pkgdown::build_site(install = FALSE)` now completes with
  clean pkgdown metadata checks.
- Added GitHub Actions workflows for default-backend R tinytests, default and
  Rust-backed installed C API checks, an aggregate native Wasmtime backend
  tinytest run, R CMD check, pkgdown Pages deployment, and the webR/wasm stub
  gate.

## Public C API

- Extended the installed R-free C API to version 2 with one deliberately narrow
  downstream boundary: `rwasmtime_runtime_call_core()`. Native backend builds
  can now compile copied core module bytes, instantiate with no imports, call
  one exported core function, and copy scalar `i32`/`i64`/`f32`/`f64` values
  through `rwasmtime_core_value_t`. The call accepts
  `rwasmtime_core_call_options_t` with explicit optional memory/table/instance,
  fuel, and wall-time limits. This does not expose components, WASI, callbacks,
  persistent stores, memory, tables, or host references. Rust-backed installs
  hide private `rwasmtime_backend_*` shim symbols and test the installed shared
  library exports only the reviewed public `rwasmtime_*` names.

## Component introspection

- Native backend builds now support real component import/export introspection
  for `wt_component_imports()` and `wt_component_exports()` when a component
  spec/prepared component has a native runtime. The result is a list of
  `WtComponentItem` objects with copied name, kind, and simple parameter/result
  schema labels. This is compile-time metadata only; component execution and
  full WIT value conversion remain future work.

## Core callbacks and jobs

- Added real low-level core callback imports for native backend builds. A
  `WtArtifact` instantiated with `WtLinker |> wt_link_callbacks()` can now bind
  core Wasm function imports to R closures declared with `wt_add_callback(...,
  abi = "core")`; scalar and multi-result values use the same dynamic Wasmtime
  value conversion as `wt_call()`.
- `wt_call_async()` now settles immediately to a completed `WtJob` for native
  calls that can already run synchronously, so `wt_await()` and `wt_result()` are
  useful on real core module calls while full background scheduling remains a
  future adapter layer.
- `wt_prepare()` now stores a native compiled core artifact for module apps and
  apps built from existing `WtArtifact` objects, so prepared calls, sessions,
  and immediate jobs reuse a real compiled module instead of recompiling source
  on every call.
- Low-level `wt_instantiate()` now rejects store/linker/runtime mismatches before
  native instantiation instead of treating store and linker runtimes as
  decorative metadata.
- R callback failures during native core Wasm calls now rethrow as
  `rwasmtime_callback_error` conditions, including through callback-backed
  sessions, low-level instances, prepared apps, and immediate jobs.
- Guest traps and failed native core Wasm calls now rethrow as `rwasmtime_trap`
  conditions while missing exports and setup errors remain ordinary errors.
- Added compact prints for `WtComponentSpec` and `WtArrayArgument`, and made
  artifact-backed app specs print an artifact label instead of list internals.
- Pending `wt_await(timeout_ms = ...)` calls now fail with a classed
  `rwasmtime_timeout` condition while no-timeout pending awaits remain honest
  `rwasmtime_not_implemented` boundaries.
- WASIp1 stdio support now covers explicit binary-safe file stdin, preloaded by
  the R/Savvy adapter into copied stdin bytes, plus `stdout = "file"` and
  `stderr = "file"` in native prepared apps and low-level linked instances.
  File stdout/stderr copy captured bytes to explicit, distinct host file paths
  after command execution.
- WASI, limits, callbacks, and callback-policy print methods now show the
  capability, limit, and callback state that matters for review instead of
  hiding important authority in list internals.
- Synchronous native core callbacks now enforce whole-number
  `wt_callback_policy(max_calls=)` and best-effort elapsed-time `timeout_ms`
  checks in the R/Savvy adapter path.
- Native `wt_memory_grow()`, `wt_memory_write()`, and `wt_array_write()` now
  check configured `WtLimits$memory_bytes` for R-triggered memory operations and
  fail with classed `rwasmtime_limit_error` conditions when the requested range
  or current memory size exceeds the limit. The same memory cap now reaches
  Wasmtime stores through a `ResourceLimiter`, so initial memories and
  guest-internal `memory.grow` beyond the cap are blocked before mutation and
  rethrown as `rwasmtime_limit_error` when observed from R.
- `wt_limit_tables()` and `wt_limit_instances()` now require whole finite
  non-negative scalars. Table element caps are enforced by the native store
  limiter; instance caps are passed to each native store created for a
  session/instance while reusable native `WtStore` handles remain future work.
- Native core module execution now wires `wt_limit_fuel()` to Wasmtime fuel and
  `wt_limit_wall_time()` to epoch interruption for prepared apps, sessions, and
  low-level instances, including start-section execution; fuel and wall-time
  limit traps are rethrown as `rwasmtime_limit_error`.

## API polish

- `wt_artifact_info()`, `wt_repl_info()`, and `wt_poll()` now return small typed
  value objects with compact print methods instead of anonymous lists.
- Added compact print methods for runtime, artifact, store, linker, memory,
  memory view, array, WASI result, REPL info, and job status objects.

## REPL protocols

- Added a real opt-in `wt_repl(protocol = "core")` path for persistent core
  module guests that expose an explicit memory ABI: `alloc(len)`,
  `eval(ptr, len)`, value pointer/length exports, and optional
  stdout/stderr/error/status/completion exports. This supports the
  MicroPython-wasm-shaped pattern without pretending to be webR or evaluating
  host R code.

## Tests

- Moved R native-backend behavior checks into installed `tinytest` files under
  `inst/tinytest/`. The Makefile backend targets now install the requested build
  mode and invoke the package test trigger in `tests/tinytest.R` with a file
  pattern; `tools/` remains for source/build/environment gates such as C API,
  Rust boundary, and wasm configure checks.

## WASI

- Added low-level WASIp1 linker support for compiled core artifacts. A native
  `WtArtifact` can now be instantiated through a `WtLinker` carrying `WtWasi`,
  and `wt_call("_start")` on the resulting `WtInstance` returns captured
  `WtWasiResult` output.

## webR/wasm build gate

- Added `make test-webr`, a non-execution webR/wasm gate that verifies configure
  selects `src/Makevars.wasm.in`, the wasm stub object is used instead of the
  native backend, and every Savvy-generated FFI symbol has a wasm fallback.

## Core call ABI

- Broadened the native dynamic core-call adapter beyond numeric scalars. Core
  calls now support exact `i64` decimal-string transport, `v128` raw vectors of
  length 16, and `NULL` for nullable reference parameters/results.

## Memory transport

- Added typed linear-memory copy helpers over the existing native raw memory
  boundary. `wt_memory_read()` / `wt_memory_write()` now support `i32`, `u32`,
  `f32`, `f64`, `i64`, and `u64` for native sessions/instances. The 64-bit
  integer paths use decimal strings to avoid silent precision loss in base R.
- Added the first real guest-array copy path. `wt_array_write()` can allocate
  through a guest `alloc` export, copy a typed payload into exported memory, and
  return a `WtArray`; `wt_as_array()` copies it back and `wt_free()` delegates to
  the guest `free` export when present.

## AOT artifacts

- Added real opt-in core AOT save/load for compiled native module artifacts.
  `wt_aot_save()` writes Wasmtime serialized module bytes plus a metadata
  sidecar, and `wt_aot_load()` validates compatibility before deserializing back
  to a reusable native artifact.

## Native adapter and backend

- Added real opt-in native core-module artifacts for the low-level object path:
  `wt_compile()` now creates a reusable compiled module handle, and
  `wt_instantiate()` creates independent persistent `WtInstance` handles for
  no-import core modules.
- Added real opt-in `wt_exec()` for persistent core sessions, low-level core
  instances, and prepared core modules. It calls a side-effecting export and
  returns the original object so low-level guest state mutations stay pipeable.
- Added persistent native core-instance/session handles through Savvy. Core
  module sessions created by `wt_new_session()` now keep one Wasmtime store and
  instance alive for repeated `wt_call()` operations.
- Moved the R-to-native bridge to generated Savvy bindings. `src/init.c`,
  `R/000-wrappers.R`, and `src/rust/api.h` are generated from `src/savvy`, with
  no post-generation registration patching.
- Split fallback native symbols by build target: default native builds use
  `src/native_stubs.c`, while webR/wasm builds use `src/wasm_stubs.c` in the
  Ropendal-style generated-symbol shim pattern.
- Added native-backend development build paths backed by real Wasmtime/Cranelift,
  including dynamic scalar core calls and WASIp1 command stdout/stderr capture.
- Core Wasm call signatures are discovered from Wasmtime `FuncType`; the package
  does not generate one native entry point per Wasm signature.

## Memory transport

- Added real R-level raw linear-memory operations for persistent native core
  sessions: `wt_memory_size()`, `wt_memory_grow()`, `wt_memory_read(dtype = "u8")`,
  and `wt_memory_write(dtype = "u8")`. Wider typed array transport remains an
  honest not-implemented boundary.

## Runtime and WASI tests

- Added real Cranelift/Wasmtime Rust tests for engine configuration and core
  module execution.
- Added real WASIp1 backend tests for deny-by-default authority, explicit
  preopens, string stdin, and captured stdout/stderr.
- Added opt-in R smoke tests for native runtime construction, `wt_call()` and
  `wt_exec()` on core modules, reusable low-level native artifacts/instances,
  core AOT save/load, persistent session memory transport, core-memory REPL
  transport, and `wt_call("_start")` on WASIp1 command modules.
- Kept the installed public C API small and R-free, with symbol-whitelist,
  boundary, header, and Rtinycc roundtrip checks.

## Package infrastructure

- Added generated Rust dependency authorship/licence notes in `inst/AUTHORS` and
  `inst/LICENCE.note`, with `tools/update-authors.R` as the source-of-truth
  generator and a DESCRIPTION copyright pointer.
- Added `make rd`, `make test-fast`, `make test-rust`, `make test-rust-backend`,
  `make test-rust-wasi`, `make test-c-api`, `make test-c-api-rust-backend`,
  `make test-r-runtime-rust-backend`, `make test-r-aot-rust-backend`,
  `make test-r-call-rust-backend`, `make test-r-exec-rust-backend`,
  `make test-r-low-level-rust-backend`,
  `make test-r-memory-rust-backend`, `make test-r-repl-rust-backend`, and
  `make test-r-wasi-rust-backend` workflows.
- Added README/vignette examples that teach the pipe-first API while preserving
  honest not-implemented boundaries for unfinished execution paths.

# Rwasmtime 0.0.0.9000

## Initial development scaffold

- Added the pipe-first R API surface for runtime specs, WASI capabilities,
  limits, callbacks, app composition, execution verbs, AOT artifacts, memory and
  array helpers, component/WIT placeholders, async jobs, and sandbox REPL
  protocols.
- Added the R-free Rust core scaffold with validated runtime configuration,
  WASI capability objects, callback broker shapes, AOT metadata compatibility,
  memory span validation, array transport policy, component/WIT value shapes,
  and REPL protocol boundaries.
- Added a tiny installed C API in `inst/include/rwasmtime.h` for version/status,
  error ownership, and runtime build/release, with no R objects crossing that
  boundary.
- Added explicit documentation that webR support is a guest protocol boundary;
  the package must not fake sandboxed webR execution by evaluating host R code.
