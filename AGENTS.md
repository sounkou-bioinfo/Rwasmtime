# Rwasmtime agent notes

## Prologue

You are working on an R package that embeds Wasmtime through a Rust core and a
thin R adapter. Keep the package sharp. Do not add fake abstraction, decorative
classes, speculative helper layers, broad dependency piles, or examples that
pretend sandbox execution exists before the Rust backend implements it. Every
public function must either be a real boundary, a stable API constructor, or a
small stub that fails with an honest `rwasmtime_not_implemented` condition.

Before landing a change, ask what boundary it affects: R API, Savvy adapter,
Rust core, Wasmtime runtime, WASI capability model, callback broker, async job
system, AOT cache/artifact layer, memory/array ABI, or sandbox REPL protocol. If
the answer is unclear, narrow the change. If the API is ambiguous, update
`design/refinement-log.md` with the ambiguity and the chosen resolution. If
implementation or tests change, update `design/STATUS.md`.

Write Rust like a runtime maintainer, not like a framework tourist. Write R like
an R package author, not like a Python notebook author. Pipe-first verbs are the
canonical R interface. Method sugar may delegate to verbs later, but examples,
tests, and documentation must teach the verb grammar. Do not paper over design
uncertainty with proliferating internal dot-helpers; if a helper is not a named,
shared policy boundary, inline the simple code or move the validation to the
Rust/Savvy boundary that owns the handle.

## Project goal

Build `Rwasmtime`: an R package exposing Wasmtime as an embeddable WebAssembly
runtime for R. The package must support:

- Low-level runtime objects: config, engine/runtime, artifacts, stores, linkers,
  instances, exports, functions, memory, tables, globals, jobs, traps.
- Higher-level app/sandbox composition: source artifact + runtime + WASI +
  limits + callbacks + array policy + REPL protocol.
- Wasm components and WIT-aware calls.
- WASI capability configuration with deny-by-default authority.
- R callbacks from Wasm through a main-thread callback broker.
- Async calls and jobs that can drain callback requests on the R main thread.
- Optional AOT artifact save/load, compiler strategy selection, optimization
  levels, cache controls, and fast-start versus optimizing compiler paths.
- SIMD128 and relaxed-SIMD feature controls, with deterministic relaxed SIMD off
  unless the user explicitly enables it.
- Explicit linear-memory and typed-array/tensor operations.
- A REPL-like sandbox protocol for guests that expose an evaluator, including a
  future webR guest adapter.

## Non-negotiable boundaries

1. `src/rust` is the runtime core. It must not depend on R objects, SEXP, ALTREP,
   or R's C API. It receives copied/serialized values and returns copied or
   owned values.
2. The R/Savvy adapter owns R closures, protection, finalizers, R conditions,
   and main-thread evaluation.
3. Background threads and async Wasmtime host functions must not call R's API.
   They may enqueue callback requests and wait for replies.
4. R callbacks from Wasm go through a callback broker. No direct R closure calls
   from worker threads. No raw SEXP crosses thread boundaries.
5. WASI authority is explicit. No inherited filesystem, environment, stdio,
   clocks, random, or network unless the API object grants it.
6. A sandbox REPL is not a Wasmtime built-in. It is a guest protocol over a
   component export, stdio command loop, or callback channel. `wt_webr_repl()` is
   a protocol adapter for a webR guest, not a license to evaluate code in host R.
7. Arrays and tensors use explicit transport: component values for small
   structured values, linear-memory buffers for large numeric payloads. Defaults
   copy data. Borrowed memory views must be visibly advanced/unsafe.
8. AOT artifacts must carry compatibility metadata. Loading an incompatible
   artifact must fail before execution.
9. Stubs must fail honestly. Do not implement examples by secretly calling host
   `eval(parse())`; that destroys the sandbox boundary.

## R API rules

- Use `.x` as the first argument for pipeable verbs.
- Teach `object |> wt_verb(...)` as the canonical API.
- Constructors use `wt_*()` nouns: `wt_runtime_spec()`, `wt_wasi()`,
  `wt_limits()`, `wt_callbacks()`, `wt_app()`.
- Mutating/composition verbs return the same conceptual object:
  `wt_with_*()`, `wt_enable_*()`, `wt_add_*()`, `wt_link_*()`.
- Finalizing verbs return built runtime objects: `wt_build_runtime()`,
  `wt_prepare()`, `wt_new_session()`.
- Terminal verbs return values or jobs: `wt_call()`, `wt_call_async()`,
  `wt_await()`, `wt_memory_read()`, `wt_as_array()`.
- Side-effect execution that keeps the pipe alive is named `wt_exec()`.
- Prefer S3 and external pointers over R6. Environment-backed objects are allowed
  only for temporary scaffold or mutable handles that will become Rust-backed.
- Avoid hidden one-line helpers with dot names unless they encode a real policy
  used in multiple places. Ridiculous helper proliferation such as
  `.wt_as_call_owner()` / `.wt_as_memory_owner()` is not allowed; handle checks
  belong in Rust/Savvy once the objects are external-pointer backed.
- Do not silently conflate missing, `NULL`, `FALSE`, `0`, and empty string. These
  are often distinct capability decisions.

## Naming conventions

Primary public types/classes:

- `WtRuntimeSpec`, `WtRuntime`
- `WtWasi`, `WtLimits`, `WtCallbacks`, `WtCallbackPolicy`
- `WtAppSpec`, `WtPreparedApp`, `WtSession`
- `WtArtifact`, `WtLinker`, `WtStore`, `WtInstance`
- `WtJob`
- `WtMemory`, `WtArray`, `WtMemoryView`
- `WtRepl`, `WtReplResult`
- `rwasmtime_error`, `rwasmtime_not_implemented`, `rwasmtime_callback_error`,
  `rwasmtime_trap`, `rwasmtime_timeout`, `rwasmtime_aot_incompatible`

Preferred public functions:

- Runtime: `wt_runtime_spec()`, `wt_with_compiler()`, `wt_enable_features()`,
  `wt_with_aot()`, `wt_with_allocator()`, `wt_build_runtime()`.
- WASI: `wt_wasi()`, `wt_wasi_args()`, `wt_wasi_env()`, `wt_wasi_preopen()`,
  `wt_wasi_stdio()`, `wt_wasi_network()`.
- Limits: `wt_limits()`, `wt_limit_memory()`, `wt_limit_tables()`,
  `wt_limit_instances()`, `wt_limit_fuel()`, `wt_limit_wall_time()`,
  `wt_limit_callbacks()`.
- Callbacks: `wt_callbacks()`, `wt_callback_policy()`, `wt_add_callback()`.
- App: `wt_app()`, `wt_as_module()`, `wt_as_component()`, `wt_with_runtime()`,
  `wt_with_wasi()`, `wt_with_limits()`, `wt_with_callbacks()`,
  `wt_with_arrays()`, `wt_with_wit()`, `wt_prepare()`.
- Execution: `wt_call()`, `wt_exec()`, `wt_call_async()`, `wt_poll()`,
  `wt_await()`, `wt_drain_callbacks()`, `wt_result()`, `wt_cancel()`.
- AOT: `wt_compile()`, `wt_aot_save()`, `wt_aot_load()`, `wt_artifact_info()`,
  `wt_artifact_compatible()`.
- Memory/arrays: `wt_memory()`, `wt_memory_size()`, `wt_memory_grow()`,
  `wt_memory_read()`, `wt_memory_write()`, `wt_memory_view()`,
  `wt_array_write()`, `wt_as_array()`, `wt_with_temp_array()`, `wt_arg_array()`,
  `wt_free()`.
- REPL: `wt_repl()`, `wt_webr_repl()`, `wt_repl_send()`, `wt_repl_read()`,
  `wt_repl_eval()`, `wt_repl_history()`, `wt_repl_info()`, `wt_repl_close()`.

## Build/development workflow

Use the Makefile targets. Do not hand-edit generated files once generation is
wired.

- `make rd`: refresh Savvy wrappers when present and roxygen docs.
- `make rdm`: install the native backend build and render `README.Rmd` to
  `README.md` with real backend examples; do not edit `README.md` independently
  after R is available.
- `make authors`: regenerate `inst/AUTHORS` and `inst/LICENCE.note` from Cargo
  metadata after Rust dependency changes.
- `make dev-install`: install current source locally.
- `make test-fast`: run non-network tinytest tests.
- `make test-rust`: verify the R-free Rust core boundary, then run default Rust tests in `src/rust`.
- `make test-rust-backend`: run the explicit feature-gated real Wasmtime/Cranelift backend smoke tests.
- `make test-rust-wasi`: run the explicit feature-gated real WASIp1 backend smoke tests.
- `make test-c-api`: use Rtinycc to compile the installed C API header and
  exercise the default C-only installed native C symbols without routing through
  R objects.
- `make test-c-api-rust-backend`: install with `RWASMTIME_RUST_BACKEND=1` and
  verify the existing C API can build/release a real Rust/Wasmtime runtime.
- `make test-r-runtime-rust-backend`: install with `RWASMTIME_RUST_BACKEND=1`
  and verify `wt_build_runtime()` returns a native Savvy runtime handle.
- `make test-r-aot-rust-backend`: install with `RWASMTIME_RUST_BACKEND=1`
  and verify core AOT save/load roundtrip plus metadata compatibility gating.
- `make test-r-call-rust-backend`: install with `RWASMTIME_RUST_BACKEND=1`
  and verify `wt_call()` executes a real core Wasm function through Wasmtime.
- `make test-r-exec-rust-backend`: install with `RWASMTIME_RUST_BACKEND=1`
  and verify `wt_exec()` keeps persistent side-effecting calls pipeable.
- `make test-r-low-level-rust-backend`: install with `RWASMTIME_RUST_BACKEND=1`
  and verify `wt_compile()`/`wt_instantiate()` produce reusable native core
  artifacts and instances.
- `make test-r-memory-rust-backend`: install with `RWASMTIME_RUST_BACKEND=1`
  and verify persistent core sessions plus copied raw/typed memory and guest
  array operations.
- `make test-r-repl-rust-backend`: install with `RWASMTIME_RUST_BACKEND=1`
  and verify the persistent core-memory REPL protocol transport.
- `make test-r-wasi-rust-backend`: install with `RWASMTIME_RUST_BACKEND=1`
  and verify prepared apps and low-level linked artifacts execute real WASIp1
  `_start` commands with captured stdout/stderr.
- `make test-r-callbacks-rust-backend`: install with
  `RWASMTIME_RUST_BACKEND=1` and verify core Wasm imports call R callbacks
  through the Savvy-owned R-main-thread boundary.
- `make test-webr`: verify wasm/webR configure selection uses
  `src/Makevars.wasm.in` and that `src/wasm_stubs.c` covers every
  Savvy-generated FFI symbol without pretending the native backend exists in
  webR.
- `make test`: install and run tinytest.
- `make check`: build and run `R CMD check --as-cran --no-manual`.
- `make clean`: remove local build products.

When Savvy changes add or rename generated `savvy_*__ffi` symbols in
`src/rust/api.h`, keep both generated-symbol fallback files synchronized:
`src/native_stubs.c` for default native no-backend builds and
`src/wasm_stubs.c` for webR/wasm builds. Do not add custom `.Call` registrations
or patch generated `src/init.c`.

Tinytest infrastructure lives in `tests/tinytest.R` and `inst/tinytest/`. Helper
code lives in `inst/tinytest/helper-rwasmtime.R`. R behavior tests, including
native-backend behavior that only runs after `RWASMTIME_RUST_BACKEND=1` installs,
belong in `inst/tinytest/` and may skip at file top level when the native backend
is absent. Keep `tools/` for source/build/environment gates or thin runners, not
for ad-hoc R behavior assertions.

## README and vignettes

`README.Rmd` is the source. `README.md` is derived output. The README must show:

- Pipe-first runtime/app/sandbox composition.
- Host callbacks from Wasm into R through `wt_callbacks()`.
- Async `wt_call_async() |> wt_await()` with callback servicing hidden behind the binding/platform machinery, not user drain loops.
- AOT/compiler feature controls.
- A sandbox REPL example, including the webR guest protocol boundary.

Vignettes must be useful, not decorative. Keep them few:

- `vignettes/api-boundaries.Rmd`: exact API grammar and ownership boundaries.
- `vignettes/sandbox-repl-webr.Rmd`: REPL protocols and webR guest adapter shape.

## What not to do

- Do not add a giant R6 surface.
- Do not add dependencies because they feel familiar.
- Do not hide Wasmtime capability grants in convenience constructors.
- Do not make examples that pretend unimplemented Rust works. Prefer executable scaffold examples that build API objects or capture honest `rwasmtime_not_implemented` errors. Use `eval = FALSE` only with a nearby explicit justification for a future guest/backend protocol sketch.
- Do not call host R as a stand-in for a sandboxed webR guest.
- Do not create both method-first and pipe-first APIs with divergent behavior.
- Do not add network, filesystem, or package-install side effects to tests.
- Do not add speculative C API sprawl. A native C surface may be introduced only
  as a small, reviewed boundary with a symbol whitelist, header compile tests,
  and clear ownership rules, not as an aspirational dump of every possible
  future runtime concept. C API exercises must use Rtinycc and must verify the
  installed header/symbols. Do not add one native entry point per Wasm
  signature; use a generic copied/tagged core-value or WIT-value boundary.
  R-facing native handles and `.Call` registrations belong in Savvy-generated
  wrappers, not post-generation patches to `src/init.c`.
