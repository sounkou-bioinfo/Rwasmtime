# Refinement log

## 2026-06-13 scaffold

Decision: make the R API pipe-first. Method syntax may be added later as sugar,
but all documentation and tests should use first-argument verbs.

Decision: include `wt_repl()` and `wt_webr_repl()` as protocol boundaries, not as
implemented execution. Wasmtime does not provide a generic REPL; the guest must
export or run one. The intended shape is an embedded guest interpreter compiled
to Wasm, analogous to a MicroPython/webR-in-Wasm runtime: R owns the ergonomic
client/session API, but evaluation state, language globals, package/module state,
stdout/stderr/display hooks, and errors live inside the Wasm guest. Do not model
this as repeated stateless calls to `_start`, and do not substitute host R
`eval(parse())` for a webR guest.

Decision: keep the Rust scaffold dependency-free until implementation work is
ready to wire Wasmtime/Savvy. This avoids fake backend behavior and makes the
core boundary visible.

## 2026-06-13 async callback servicing

Ambiguity: earlier scaffold text treated callback draining as something normal R
users would do with `wt_drain_callbacks()` inside polling loops.

Resolution: that is the wrong public contract. Rwasmtime should follow the
Rtinycc-style machinery: worker-capable code schedules callbacks to the R main
thread; POSIX builds wake R through an input handler/pipe, Windows builds use the
message pump, and blocking callback returns use native synchronization while the
binding layer services callbacks. `wt_await()` and generated/bound wrappers must
hide that machinery. `wt_drain_callbacks()` remains only an advanced adapter-test
or tight-native-loop hook, not part of examples or normal user control flow.

## 2026-06-13 executable documentation

Decision: do not use blanket `eval = FALSE` in `README.Rmd` or vignettes. Examples
should execute against the scaffold when possible, or explicitly capture the
honest `rwasmtime_not_implemented` boundary. Non-R shell commands can be shown as
shell fences. Future-backend protocol sketches must be justified in prose rather
than hidden behind unqualified skipped chunks.

## 2026-06-14 core call ABI

Ambiguity: the first real R `wt_call()` smoke bridge used a signature-specific
native function name for an `(i32, i32) -> i32` test path. That would scale in the
wrong direction: one FFI/R adapter function per Wasm signature.

Resolution: remove all signature-specific call bridge symbols and use one generic
copied/tagged core-value boundary. The R/Savvy adapter converts scalar numeric
values at the typed boundary, the installed public C API does not expand, and
Rust inspects the Wasmtime export type to coerce/check arguments and copy
results back. Future WIT/component calls should follow the same principle with a
WIT-value tree, not function-per-signature glue.

## 2026-06-14 Savvy adapter ownership

Ambiguity: the first R bridge used hand-written `.Call` entry points patched into
Savvy-generated `src/init.c`. That violated the Ropendal/Rzarrs pattern and made
registration ownership unclear.

Resolution: `src/init.c`, `R/000-wrappers.R`, and `src/rust/api.h` are generated
by Savvy from the adapter crate in `src/savvy`. The R-facing runtime handle is a
Savvy class (`RwasmtimeNativeRuntime`) and the default C-only build supplies only
Ropendal-style fallback implementations of the generated `savvy_*__ffi` symbols.
There is no post-generation init patch and no custom R `.Call` registration
layer. Dynamic Wasm call signatures are discovered from Wasmtime `FuncType` at
call/module load time; do not use TinyCC or one generated native function per
Wasm signature for this boundary.

Follow the Ropendal webR split: default native no-backend builds may provide
fallback implementations of generated `savvy_*__ffi` symbols, but `src/wasm_stubs.c`
is reserved for webR/wasm fallback implementations selected by configure target
detection. Native fallback code lives separately in `src/native_stubs.c`.

## 2026-06-14 persistent core sessions

Decision: expose persistent core-module execution through a generated Savvy
`RwasmtimeNativeInstance` owned by `WtSession`, not by expanding the installed C
API or hand-registering `.Call` functions. `wt_new_session()` may instantiate a
native core module when a native runtime is available, and `wt_call()` on that
session reuses the same Wasmtime store/instance. Raw `u8` linear-memory
read/write/grow/size operations are a real copied boundary on that persistent
instance. Wider typed arrays, borrowed views, component memories, and WASI
interactive sessions remain separate explicit boundaries.

## 2026-06-14 core-memory REPL protocol

Decision: add a narrow real REPL protocol for core modules before full
component/WIT or interactive WASI stdio exists. `wt_repl(protocol = "core")`
models a persistent guest interpreter that exposes exported memory plus an
explicit ABI: allocate or otherwise choose an input pointer, copy UTF-8 source
into guest memory, call `eval(ptr, len)`, then read UTF-8 value bytes from
value/result pointer-length exports. Guests may also expose stdout, stderr,
error, status, and completion pointer/value exports so `WtReplResult` is
structured instead of a single sloppy string. This matches the
MicroPython-wasm-shaped guest VM pattern without inventing a language runtime in
the host. It is not webR; the future webR adapter can use this kind of explicit
protocol or a component/stdio one, but must still run evaluation inside the
guest.

## 2026-06-14 primitive-first API bias

Decision: prefer low-level core Wasm primitives for maximal power. Component/WIT
should be an optional typed transport layered above persistent instances,
explicit function calls, and copied memory operations, not a replacement for
those primitives. When choosing the next backend slice, implement the primitive
first (`wt_call()`, `wt_exec()`, memory reads/writes, explicit protocol exports),
then add ergonomic typed wrappers later.

Decision: low-level artifacts should become real before high-level convenience
layers. Native `wt_compile()` now means a compiled core module handle when the
opt-in backend is linked; `wt_instantiate()` creates fresh persistent instances
from that handle. This keeps module compilation, instantiation, calls, side
effects, and memory transport visible as separate user-controllable boundaries.

Decision: AOT save/load is a low-level artifact operation with compatibility
metadata, not an opaque cache. The native backend serializes Wasmtime compiled
module bytes and stores package/runtime metadata in a sidecar. Loading validates
metadata before calling Wasmtime's unsafe deserialize API; incompatible or
metadata-less artifacts fail before execution.

Decision: typed linear-memory transport is copy-first and exact where base R can
be exact. `u8` remains raw-vector transport. `i32`, `u32`, `f32`, and `f64` use
ordinary R vectors. `i64` and `u64` use decimal strings instead of doubles so
large values are not silently rounded. Borrowed views and `v128` remain explicit
future/advanced boundaries.

Decision: dynamic core-call ABI should use the same exactness policy as typed
memory. `i64` parameters/results use decimal strings at the R boundary, `v128`
uses raw length-16 payloads, and nullable references accept/return structured
null values. Non-null host references are not smuggled through R; they require a
future explicit handle/import design.

Decision: the first array helper should remain a visible copy over guest linear
memory, not a hidden view or host arena. `wt_array_write()` requires a guest
allocator export and returns a `WtArray` descriptor carrying pointer, byte
length, dtype, dim, and free policy. Borrowed views and host arenas remain
future advanced boundaries.

Decision: low-level WASI should be a linker capability, not only a prepared-app
shortcut. A `WtLinker` carrying explicit `WtWasi` can instantiate a compiled
core artifact with WASIp1 imports. Captured stdout/stderr remain copied output
queried through the instance after `_start`; no ambient WASI authority is added.

## 2026-06-14 core callback imports

Decision: implement the first callback backend at the low-level core import
boundary. `WtLinker |> wt_link_callbacks()` now binds core Wasm function imports
to R closures for synchronous calls initiated from the R main thread. The Rust
core remains R-free by accepting generic Wasmtime host-function closures; the
Savvy adapter owns and preserves R closures, converts copied Wasm values to R
arguments, evaluates the R callback, and copies results back to Wasmtime. This is
not the future worker-thread callback broker: async/background host functions
must still enqueue requests and wake/service R on the main thread instead of
calling R directly.

Decision: `wt_call_async()` may settle immediately for native calls that can run
now. This gives `WtJob`, `wt_poll()`, `wt_await()`, and `wt_result()` real value
on supported core calls without claiming background scheduling. The later async
job system can reuse the same public job state once worker execution and callback
servicing are implemented.

## 2026-06-15 installed C API v2

Ambiguity: downstream packages need an `inst/include/rwasmtime.h` boundary like
Ropendal, but expanding the C API can easily become Potemkin sprawl or a way to
bypass the Savvy/R adapter.

Resolution: expose exactly one additional public C execution primitive for now:
`rwasmtime_runtime_call_core()`. It is a one-shot no-import core-module call over
copied module bytes and copied scalar `rwasmtime_core_value_t` values, with an
optional `rwasmtime_core_call_options_t` carrying explicit resource/fuel/wall-time
limits. It deliberately does not expose components, WASI, callbacks, persistent
stores/instances, memory, tables, host references, or R objects. R-level behavior
continues through the generated Savvy adapter; the C API is for downstream native
packages that include the installed header and link/load `Rwasmtime.so`.
