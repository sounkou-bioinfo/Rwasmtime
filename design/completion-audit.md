# Rwasmtime scaffold-phase completion audit

Date: 2026-06-13 / 2026-06-14 UTC

This file records the earlier scaffold/API+C-API completion audit. It is not a
claim that the full Wasmtime/webR objective is complete. Since this audit, the
project has moved into backend implementation slices: generated Savvy runtime
handles, opt-in Wasmtime/Cranelift calls, WASIp1 command execution, persistent
core sessions, raw `u8` memory operations, Ropendal-style wasm/native stubs, and
generated authorship/licence notes. See `design/STATUS.md` for current backend
status and remaining work.

## Objective restated as concrete deliverables

User objective: "implement all the intended api including r free C API exercice too".

Concrete completion criteria for the current scaffold phase:

1. Provide the intended pipe-first R API surface named in `AGENTS.md`, exported and documented from roxygen-generated sources.
2. Keep backend operations honest: unimplemented Wasmtime execution, memory, callback, AOT, component, and REPL operations must fail with `rwasmtime_not_implemented` or equivalent scaffold/runtime conditions rather than fake execution.
3. Provide R-level validation for capability and policy objects where it protects API ambiguity before the Rust/Savvy backend lands.
4. Provide an R-free Rust core scaffold matching the intended runtime boundaries: runtime config, WASI, limits, callbacks, app/session/job, AOT metadata, low-level runtime objects, memory/array transport, components/WIT, and REPL protocol.
5. Enforce that `src/rust/src` remains R-free: no Savvy, R API, SEXP, ALTREP, or R-object dependencies in the core.
6. Provide the minimal reviewed R-free C API exercise: installed header, implementation stubs, symbol whitelist, R-free boundary scan, Rtinycc header compile, and installed-symbol roundtrip.
7. Keep package layout Ropendal-style: `README.Rmd` source plus generated `README.md`, generated `NAMESPACE`/Rd via `make rd`, vignettes, tinytest tests, Makefile workflow, and repo-only development files excluded from source tarballs where required.
8. Pass project validation commands: `make rd`, `make rdm`, `make test-rust`, `make test-fast`, `make test-c-api`, and `make check`.

## Prompt-to-artifact checklist

| Requirement | Artifact/evidence | Status |
|---|---|---|
| Pipe-first intended R API | `R/api.R`; `NAMESPACE`; `man/Rwasmtime-api.Rd`; `tools/check-api-surface.R` checks 64 preferred + 9 scaffold public functions | Complete: `make test-fast` ran `API surface check ok: 64 preferred public, 9 scaffold public, 7 internal helpers` |
| Honest unimplemented backend boundaries | `R/api.R`; Rust modules in `src/rust/src`; tinytests in `inst/tinytest`; Rust tests | Complete: R/tinytest references confirm `rwasmtime_not_implemented`; Rust test `execution_boundaries_fail_honestly_until_backend_lands` and related tests passed |
| R validation for policy/capability ambiguity | `R/api.R`; tests in `inst/tinytest/test-*.R` | Complete: `make test-fast` passed 93 tinytest checks across callbacks, config, low-level, memory/arrays, component/WIT, and REPL |
| R-free Rust core scaffold | `src/rust/src/{config,wasi,limits,callbacks,app,aot,runtime_objects,arrays,component,repl,lib}.rs` | Complete: `make test-rust` compiled and passed 45 Rust tests |
| Rust core R-free boundary enforced | `tools/check-rust-core-boundary.R`; `make test-rust-boundary`; `make test-rust` depends on it | Complete: `R-free Rust core boundary check ok: 11 source files` before Rust tests |
| Minimal reviewed C API exercise | `inst/include/rwasmtime.h`; `src/c_api.c`; `tools/check-c-api-symbols.R`; `tools/check-c-api-boundary.R`; `tools/check-c-api-header.R`; `tools/check-c-api-roundtrip.R` | Complete: `make test-c-api` passed symbol, R-free boundary, Rtinycc header compile, install, and roundtrip checks |
| C API not speculative | `tools/check-c-api-symbols.R`; `tools/check-c-api-rust-backend.R`; installed shared-library `nm` audit | Complete: current C API v2 whitelist reports `C API symbol whitelist check ok: 8 symbols`; the added symbol is the narrow installed `rwasmtime_runtime_call_core()` boundary with copied bytes/scalar values and explicit limits. Rust-backed installs also audit that only the reviewed public `rwasmtime_*` symbols are exported, so private `rwasmtime_backend_*` shims do not become accidental ABI. |
| Ropendal-style docs/layout | `README.Rmd`; `README.md`; `_pkgdown.yml`; `vignettes/`; `.Rbuildignore`; `Makefile`; generated docs | Complete: `make rd` regenerated Savvy/roxygen outputs; `make rdm` regenerated `README.md`; `make check` built vignettes and passed examples/tests |
| README.Rmd and `_pkgdown.yml` excluded from source tarball | `.Rbuildignore`; `R CMD build .`; tarball listing | Complete: tarball grep showed `Rwasmtime/inst/include/rwasmtime.h` and `Rwasmtime/src/rust/src/lib.rs`, with no `README.Rmd`, `_pkgdown.yml`, or `tools/` entries |
| No unqualified skipped examples | grep over `README.Rmd`, `README.md`, and `vignettes/` for `eval = FALSE` | Complete: grep returned no matches |
| Package validation | `make rd`; `make rdm`; `make test-rust`; `make test-fast`; `make test-c-api`; `make check` | Complete: all commands ran successfully; `make check` ended `Status: 2 NOTEs` with expected dev-version/placeholder-URL and local compiler-flag NOTEs, no warnings/errors |

## Final command evidence

Final validation command executed:

```sh
make rd
make rdm
make test-rust
make test-fast
make test-c-api
make check
```

Observed evidence:

- `make rd`: parsed all Rust scaffold modules, wrote `src/rust/api.h`, `src/init.c`, and `R/000-wrappers.R`; roxygen completed.
- `make rdm`: rendered `README.Rmd` to `README.md` successfully.
- `make test-rust`: `R-free Rust core boundary check ok: 11 source files`; `45 passed; 0 failed` Rust tests.
- `make test-fast`: `API surface check ok: 64 preferred public, 9 scaffold public, 7 internal helpers`; tinytest `All ok, 93 results`.
- `make test-c-api`: current runs report `C API symbol whitelist check ok: 8 symbols`; `R-free C API boundary check ok`; `Rtinycc C API header compile check ok`; `Rtinycc C API roundtrip check ok` against the installed header/library. `tools/check-c-api-rust-backend.R` additionally reports `installed C API symbol audit ok: 8 rwasmtime_* symbols` before calling a real Wasm binary through the installed Rust-backed C API.
- `make check`: package built, installed, examples/tests/vignettes passed, and finished with `Status: 2 NOTEs` only.

Additional direct checks:

```sh
grep -R "eval *= *FALSE\|eval=FALSE\|eval = FALSE" -n README.Rmd README.md vignettes
```

No matches.

```sh
tar tzf Rwasmtime_0.0.0.9000.tar.gz | grep -E '(^Rwasmtime/inst/include/rwasmtime\.h$|^Rwasmtime/src/rust/src/lib\.rs$|README\.Rmd|_pkgdown\.yml|^Rwasmtime/tools/)'
```

Returned only:

```text
Rwasmtime/inst/include/rwasmtime.h
Rwasmtime/src/rust/src/lib.rs
```

This confirms required package artifacts are included while repo-only `README.Rmd`, `_pkgdown.yml`, and `tools/` are excluded.

## Known non-goals / deferred backend work

The real Wasmtime backend is intentionally not implemented in this scaffold phase. `design/STATUS.md` keeps a separate backend checklist for future work: adding `wasmtime`, `wasmtime-wasi`, and real `savvy` dependencies; replacing list/env handles with Rust-backed external pointers; implementing real compile/instantiate/call/memory/component/WASI/AOT/callback behavior. Current public API stubs must fail honestly until those boundaries are implemented.

## Completion conclusion

The scaffold/API+C-API exercise phase was complete at the time of this audit. All named API surfaces, boundary gates, docs, examples, package layout, and validation commands had concrete evidence. The broader runtime objective remains open and is tracked in `design/STATUS.md`.

---

# Settle-API progress audit

Date: 2026-06-14 UTC

This audit responds to the active thread objective: "implement the settle api,
call into pi for reviews as needed". It is intentionally a progress audit, not a
completion claim.

## Objective restated as concrete deliverables

For this thread, "settle API" has meant making the public Rwasmtime API less
scaffold-like and more honest/user-facing while preserving the project
boundaries in `AGENTS.md`:

1. Keep the canonical pipe-first `wt_*` API and generated Savvy bridge intact.
2. Class user-facing status/info/result/error objects instead of leaking
   anonymous lists where public calls return structured state.
3. Improve print methods so capability, runtime, artifact, job, memory, REPL,
   and result objects expose review-relevant state compactly.
4. Enforce capability and policy validation, especially WASI stdio, callback
   policy, and resource limits.
5. Replace scaffold-only paths with real opt-in native backend behavior where a
   narrow backend boundary is ready.
6. Preserve honest `rwasmtime_not_implemented` boundaries for unfinished paths;
   do not fake component/WIT, webR, background async, borrowed views, or worker
   callback brokering.
7. Call Pi/reviewer help on non-trivial slices and revise when concrete
   findings are raised.
8. Validate both default no-backend/webR fallback builds and opt-in native
   backend behavior.

## Prompt-to-artifact checklist

| Requirement | Artifact/evidence inspected | Current status |
|---|---|---|
| Pipe-first public API remains canonical | `R/api.R`, `tools/check-api-surface.R`, `make help` target list | Maintained. Public examples/docs still use `wt_*` verbs and Makefile gates cover API surface. |
| Generated Savvy bridge intact | `make rd`; generated `R/000-wrappers.R`, `src/init.c`, `src/rust/api.h`; `src/savvy/src/lib.rs` | Maintained. Latest signature changes were regenerated with `make rd`; default/webR stubs were updated to match generated prototypes. |
| Classed status/info/result/error objects | `R/api.R`; tests in `inst/tinytest/`; grep evidence for `WtArtifactInfo`, `WtReplInfo`, `WtJobStatus`, `WtWasiResult`, `rwasmtime_*` errors | Substantially implemented for current public objects: artifact info, REPL info/results, job status, WASI results, callback/trap/timeout/AOT/limit errors. |
| Compact print methods | `R/api.R`; `inst/tinytest/test-config-pipes.R`; native backend tests | Substantially implemented for runtime specs/runtimes, artifacts, stores/linkers/instances/sessions, jobs/status, memory/views/arrays/array args, app/component specs, REPL handles/info/results, WASI results/specs, limits, callbacks, and callback policies. |
| WASI capability/stdio validation | `R/api.R`; `inst/tinytest/test-config-pipes.R`; `inst/tinytest/test-rust-backend-wasi.R` | Implemented for current stdio modes, distinct file sinks, adapter-preloaded binary-safe stdin file support, unsupported dots, scalar path validation, and explicit post-exec file copies. Native streaming stdio remains pending. |
| Callback policy enforcement | `R/api.R`; `inst/tinytest/test-rust-backend-callbacks.R`; reviewer findings fixed | Implemented for synchronous native core callbacks: whole-number `max_calls`, max-depth/reentrant checks, timeout checks, shared wrapper state for duplicate imports, and `rwasmtime_callback_error` classification. Worker-thread broker remains pending. |
| Resource/limit enforcement | `R/api.R`; `src/rust/src/backend.rs`; `src/savvy/src/lib.rs`; `inst/tinytest/test-rust-backend-call.R`; `inst/tinytest/test-rust-backend-memory.R` | Implemented for R-triggered memory range/current-size checks, Wasmtime store-limiter memory prevention for initial memory and guest-internal growth, table element caps, per-native-store instance caps crossing the Savvy boundary, and native fuel/wall-time store limits including start-section execution. Store-wide accounting across repeated uses of the same R `WtStore` spec and background cancellation remain pending. |
| Real native backend replacement of scaffold paths | `src/rust/src/backend.rs`; `src/savvy/src/lib.rs`; native tinytests; installed C API probes | Implemented for core module compile/instantiate/call/exec, persistent sessions, low-level artifacts, AOT save/load, WASIp1 prepared/low-level execution, core callbacks, typed memory, guest arrays, core-memory REPL, component metadata introspection, and a narrow installed C API one-shot no-import core-call boundary. |
| Honest unfinished boundaries | `rg` over `R/api.R`, `README.Rmd`, `design/STATUS.md`, tests | Maintained. Native component metadata introspection is implemented, but Component/WIT dynamic calls, true background async, worker callback broker, reusable native store handles/store-wide accounting, borrowed views/host arenas, v128 memory lanes, WASIp2/component WASI, streaming stdio, and webR guest execution remain explicitly future/not implemented. |
| Reviewer/PI help used | Reviewer subagent calls recorded in conversation; fixes reflected in code/tests | Used. Callback-policy reviewer finding (fractional `max_calls`) and execution-limit reviewer findings (stub prototype drift, start-section limits, wall-time integer truncation) were fixed. Latest follow-up review was attempted but blocked by Pi/Codex usage limits; self-validation covered the concrete findings. |
| Default and webR fallback validation | `make test-webr`; `make test-fast`; `tools/check-webr-wasm-gate.R` | Passed in latest run. `check-webr-wasm-gate.R` now checks generated Savvy FFI prototypes, not only symbol names. |
| Native backend validation | Native Makefile targets recorded in conversation and `design/STATUS.md` | Full native matrix passed after the latest WASI/stdin and limit changes: runtime, call, callbacks, exec, low-level, memory, REPL, AOT, and WASI targets all ran successfully. |
| Package validation | `make check` output | Passed after latest changes with the expected 2 NOTEs and no warnings/errors. |

## Real evidence inspected in this audit

Commands/files inspected in this audit pass:

- `design/completion-audit.md`: older scaffold audit explicitly says full
  Wasmtime/webR objective remains open.
- `design/STATUS.md`: backend checklist still has partial and unchecked items,
  including component/WIT dynamic value conversion and REPL protocol completion.
- `rg` over `R`, `inst/tinytest`, `README.Rmd`, `design`, `NEWS.md`,
  `src/rust/src`, and `src/savvy/src`: confirmed remaining honest future/not
  implemented boundaries for component/WIT, worker callback brokering,
  background async, borrowed/v128 memory paths, streaming stdio, and webR guest
  execution.
- `make help` / `Makefile`: confirms current validation target matrix and native
  backend targets.
- Latest command evidence recorded during this thread: `cargo check
  --manifest-path=src/savvy/Cargo.toml`, `cargo check
  --manifest-path=src/rust/Cargo.toml --features c-api,wasi`, targeted Rust
  callback/WASI/store-limiter/component-introspection tests, full `cargo test
  --features wasmtime`, `make test-webr`, `make test-fast`, the full native
  backend matrix, and `make check` passed.

## Completion conclusion

The settle-API work is materially advanced but not complete. The current state
meets many settle criteria for the low-level core Wasm/native backend and public
R object polish, but explicit required work remains in `design/STATUS.md` and in
honest `rwasmtime_not_implemented` paths. Therefore the active goal must remain
open; do not call `update_goal` yet.

Recommended next concrete work:

1. Pick the next unfinished real-backend boundary: component/WIT value
   conversion, true background async/callback broker, native streaming WASI
   stdio, borrowed memory views/host arenas, or webR guest protocol execution.
2. Re-run the relevant focused target plus the full native backend matrix after
   any further backend-wide changes.
3. Update this audit only after remaining requirements have concrete file and
   command evidence.
