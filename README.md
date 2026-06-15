
<!-- README.md is generated from README.Rmd. Please edit this file. -->

# Rwasmtime

<!-- badges: start -->

[![Lifecycle:
experimental](https://img.shields.io/badge/lifecycle-experimental-orange.svg)](https://lifecycle.r-lib.org/articles/stages.html#experimental)
<!-- badges: end -->

Rwasmtime: **pipe-first Wasmtime embedding for R** via a Rust runtime
core and a thin [`savvy`](https://github.com/yutannihilation/savvy)
adapter. The intended bottom layer is Wasmtime: engines, stores,
linkers, modules, components, WASI, linear memory, async jobs, AOT
artifacts, and callback broker state live in Rust. The R layer owns R
objects, R closures, conditions, finalizers, and main-thread callback
evaluation.

The package is intentionally capability-explicit. WASI authority is
denied by default; filesystem preopens, environment variables, stdio,
network, clocks, randomness, callbacks, memory transport, and resource
limits must appear in the pipe when they are granted.

## Implementation status

Rwasmtime exposes the pipe-first API now while the native runtime
surface is being filled in. The design starts with primitives rather
than a pretend high level sandbox: build a runtime, prepare a module,
call a named export, link WASI only when authority is explicit, and move
larger values through linear memory or compiled artifacts. When an
installed build does not contain a particular execution path, the API
raises a typed `rwasmtime_not_implemented` condition instead of
simulating Wasm with host R.

Native backend builds already exercise the real Wasmtime boundary for
core modules, low-level core callback imports, immediately settled jobs
for supported native calls, WASIp1 command modules, AOT artifacts,
persistent stores/instances, copied typed memory, and a small
core-memory REPL protocol. Component calls, background job scheduling,
worker-thread callback brokering, borrowed memory views, host arenas,
WIT conversion, interactive WASI stdio, and a webR guest adapter remain
explicit future work. The webR path must be a persistent guest
interpreter protocol, not host R `eval(parse())`.

## Installation from a source checkout

From a source checkout:

``` sh
# install.packages("remotes")
R -q -e 'remotes::install_local(".", dependencies = TRUE)'
```

## A Quick Start

The API grammar is deliberately left-to-right:

``` r
library(Rwasmtime)

rt <- wt_runtime_spec() |>
  wt_with_compiler(
    strategy = "cranelift",
    opt_level = "speed",
    parallel = TRUE
  ) |>
  wt_enable_features(
    component_model = TRUE,
    simd = TRUE,
    relaxed_simd = FALSE,
    relaxed_simd_deterministic = FALSE,
    bulk_memory = TRUE,
    multi_memory = TRUE,
    memory64 = FALSE,
    threads = FALSE
  ) |>
  wt_with_aot(cache = TRUE) |>
  wt_with_allocator(strategy = "on_demand") |>
  wt_build_runtime()

rt
#> <WtRuntime> backend=native
#>   compiler: cranelift opt=speed parallel=TRUE
#>   features: component_model=TRUE simd=TRUE relaxed_simd=FALSE memory64=FALSE threads=FALSE exceptions=FALSE legacy_exceptions=FALSE
```

A minimal core Wasm call is just a module source, a runtime,
preparation, and a named export. The example uses WebAssembly text
format so the complete guest is visible in the README. In a native
backend build this call returns `42`; in a lightweight build it returns
the typed not-implemented boundary instead of pretending to execute.

``` r
add_wat <- '
(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add))
'

add_app <- wt_app(add_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_prepare()

add_app |> wt_call("add", 20L, 22L)
#> [1] 42
```

WASI and limits are separate capability objects. Nothing is inherited
silently.

``` r
wasi <- wt_wasi() |>
  wt_wasi_args("--input", "/data/input.csv") |>
  wt_wasi_env(TZ = "UTC", MODE = "test") |>
  wt_wasi_preopen(guest = "/data", host = tempdir()) |>
  wt_wasi_stdio(stdin = "empty", stdout = "capture", stderr = "capture") |>
  wt_wasi_network(FALSE)

limits <- wt_limits() |>
  wt_limit_memory("512MiB") |>
  wt_limit_tables(elements = 20000) |>
  wt_limit_instances(32) |>
  wt_limit_fuel(1e8) |>
  wt_limit_wall_time(ms = 5000) |>
  wt_limit_callbacks(max_calls = 10000, timeout_ms = 1000)

wt_readme_stable_print(wasi)
#> <WtWasi> args=2 env=2 preopens=1
#>   stdio: stdin=empty stdout=capture stderr=capture
#>   preopens: /data=><tempdir> (ro)
#>   ambient: network=FALSE clocks=FALSE random=FALSE
limits
#> <WtLimits> memory=536870912 tables=20000 instances=32
#>   execution: fuel=1e+08 wall_time_ms=5000
#>   callbacks: max_calls=10000 timeout_ms=1000 max_depth=1 reentrant=FALSE
```

Callbacks are explicit imports. For core modules, the linker binds a
Wasm function import to an R closure owned by the Savvy adapter. The
call remains a Wasm call: the guest imports `r.add_one`, calls it, and
receives a copied Wasm `i32` result.

``` r
callback_wat <- '
(module
  (import "r" "add_one" (func $add_one (param i32) (result i32)))
  (func (export "run") (param i32) (result i32)
    local.get 0
    call $add_one
    i32.const 40
    i32.add))
'

callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "add_one",
    fun = function(x) as.integer(x + 1L),
    params = "i32",
    results = "i32",
    abi = "core"
  )

callback_artifact <- rt |>
  wt_compile(callback_wat, kind = "module")

callback_instance <- callback_artifact |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(callbacks)
  )

callback_instance |> wt_call("run", 1L)
#> [1] 42
```

Structural binding metadata can be extracted from a compiled core
artifact. This is the declarative skeleton: it shows functions,
memories, tables, globals, tags, and Wasm value types, but it does not
infer that an `i32` is a pointer, string, array handle, or owned
resource.

``` r
wt_item_table <- function(items) {
  data.frame(
    direction = vapply(items, `[[`, character(1), "direction"),
    module = vapply(items, function(x) if (is.null(x$module)) "" else x$module, character(1)),
    name = vapply(items, `[[`, character(1), "name"),
    kind = vapply(items, `[[`, character(1), "kind"),
    signature = vapply(items, `[[`, character(1), "signature"),
    row.names = NULL
  )
}

wt_item_table(callback_artifact |> wt_imports())
#>   direction module    name     kind      signature
#> 1    import      r add_one function (i32) -> (i32)
wt_item_table(callback_artifact |> wt_exports())
#>   direction module name     kind      signature
#> 1    export         run function (i32) -> (i32)

callback_artifact |> wt_bindings()
#> <WtBindings> imports=1 exports=1
```

The same objects compose into higher-level app specs. Component/WIT
calls will sit above these primitives, but the low-level module, linker,
callback, memory, and WASI boundaries are the stable foundation.

``` r
plugin <- wt_app(add_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(limits) |>
  wt_with_arrays(
    default_dtype = "f64",
    layout = "column-major",
    transport = "memory"
  ) |>
  wt_prepare()

plugin
#> <WtPreparedApp> kind=module backend=native artifact=TRUE
plugin |> wt_call("add", 20L, 22L)
#> [1] 42
```

## Async and callback servicing

Async execution is a job pipeline. Native calls that can complete
immediately already settle into a done `WtJob`; future background
execution will use the same job object while the adapter services
callbacks on the R main thread.

``` r
job <- plugin |>
  wt_call_async("add", 20L, 22L)

wt_poll(job)
#> <WtJobStatus> export=add state=done done=TRUE cancelled=FALSE error=FALSE

job |> wt_await()
#> [1] 42
```

Manual polling can still stay pipeable, but it should not expose
callback servicing as a user responsibility:

``` r
for (tick in seq_len(3)) {
  status <- job |>
    wt_poll()

  if (status$done) break
  Sys.sleep(0.001)
}

job |>
  wt_result()
#> [1] 42
```

`wt_drain_callbacks()` is reserved for adapter tests or unusual tight
native loops that never yield to R’s event loop. The real backend should
mirror the Rtinycc cross-platform callback machinery: POSIX input
handlers, Windows message pumping, and worker-thread synchronization
hidden behind the binding layer.

## AOT artifacts

Artifacts are pipe-native. In native backend builds, `wt_compile()`
creates a reusable compiled core-module handle for no-import modules and
`wt_aot_save()` can serialize that compiled module to disk. A metadata
sidecar is written beside the artifact, and `wt_aot_load()` validates
compatibility before deserializing. `wt_aot_save()` still returns the
artifact so the pipeline can keep moving.

``` r
artifact <- rt |>
  wt_compile("stats_plugin.component.wasm", kind = "component") |>
  wt_aot_save(file.path(tempdir(), "stats_plugin.cwasm"), overwrite = TRUE)

wt_readme_stable_print(artifact |> wt_artifact_info())
#> <WtArtifactInfo> kind=component backend=pending
#>   input: stats_plugin.component.wasm
#>   aot_path: <tempdir>/stats_plugin.cwasm
#>   compiler: cranelift opt=speed
#>   features: component_model=TRUE simd=TRUE relaxed_simd=FALSE memory64=FALSE threads=FALSE exceptions=FALSE legacy_exceptions=FALSE

plugin_from_artifact <- wt_app(artifact) |>
  wt_with_runtime(rt) |>
  wt_with_wasi(wasi) |>
  wt_with_callbacks(callbacks) |>
  wt_prepare()

plugin_from_artifact
#> <WtPreparedApp> kind=artifact backend=pending artifact=TRUE
```

In native backend builds, the low-level object path and core AOT
roundtrip are real for no-import core modules. Dynamic core calls use
copied values: numeric scalars, exact `i64` decimal strings, `v128` raw
vectors, and `NULL` for nullable references.

``` r
add_wat <- '
(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add))
'

path <- file.path(tempdir(), "add.cwasm")
artifact <- rt |>
  wt_compile(add_wat, kind = "module") |>
  wt_aot_save(path, overwrite = TRUE)
loaded <- rt |> wt_aot_load(path)
instance <- loaded |>
  wt_instantiate(store = rt |> wt_store(), linker = rt |> wt_linker())
instance |> wt_call("add", 20L, 22L)
#> [1] 42
```

Low-level WASIp1 imports are linked by putting an explicit `WtWasi`
object on the linker. The module below writes a fixed string to stdout
through `wasi_snapshot_preview1.fd_write`; no ambient stdio or
filesystem authority is inherited.

``` r
echo_wasi_wat <- '
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 32) "hello from wasi\\n")
  (func (export "_start")
    (i32.store (i32.const 0) (i32.const 32))
    (i32.store (i32.const 4) (i32.const 16))
    (drop (call $fd_write
      (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 16)))))
'

wasi_artifact <- rt |> wt_compile(echo_wasi_wat, kind = "module")
wasi_linker <- rt |>
  wt_linker() |>
  wt_link_wasi(wt_wasi() |> wt_wasi_stdio(stdout = "capture", stderr = "capture"))
wasi_instance <- wasi_artifact |>
  wt_instantiate(store = rt |> wt_store(), linker = wasi_linker)
wasi_instance |> wt_call("_start")
#> <WtWasiResult> stdout_bytes=16 stderr_bytes=0
#>   stdout: hello from wasi
```

## Linear memory and arrays

Large numeric objects should move through explicit buffers, not hidden R
object serialization. Defaults copy data. Borrowed views must be visibly
advanced and must carry an explicit lifetime. Native backend builds
expose copied linear-memory transport for raw bytes and typed numeric
lanes on persistent core-module sessions.

``` r
session <- plugin |>
  wt_new_session()

session |>
  wt_array_write(
    c(1, 2, 3),
    dtype = "f64",
    layout = "column-major",
    allocator = "guest"
  )
#> Error:
#> ! failed to resolve memory export `memory`
```

In native backend builds, memory operations share state with repeated
calls on the same session. `u8` uses raw vectors; `i32`, `u32`, `f32`,
and `f64` copy to ordinary R numeric vectors; `i64` and `u64` use
decimal strings to avoid silently losing precision in base R doubles.

``` r
memory_wat <- '
(module
  (memory (export "memory") 1)
  (data (i32.const 0) "abc")
  (func (export "load8") (param i32) (result i32)
    local.get 0
    i32.load8_u)
  (func (export "store8") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8))
'

session <- wt_app(memory_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_prepare() |>
  wt_new_session()

mem <- session |> wt_memory("memory")
mem <- mem |> wt_memory_write(ptr = 1, value = charToRaw("Z"), dtype = "u8")
as.data.frame(list(
  bytes = rawToChar(mem |> wt_memory_read(ptr = 0, length = 3, dtype = "u8")),
  load8 = session |> wt_call("load8", 1L)
))
#>   bytes load8
#> 1   aZc    90
```

A `WtArray` is an explicit guest-memory copy, not a hidden R view. The
guest must provide an allocator and, if desired, a matching free
function.

``` r
array_wat <- '
(module
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 256))
  (func (export "alloc") (param $n i32) (result i32)
    (local $ptr i32)
    global.get $heap
    local.set $ptr
    global.get $heap
    local.get $n
    i32.add
    global.set $heap
    local.get $ptr)
  (func (export "free") (param i32 i32))
  (func (export "sum_f64") (param $ptr i32) (param $n i32) (result f64)
    (local $i i32)
    (local $sum f64)
    (loop $loop
      local.get $i
      local.get $n
      i32.lt_s
      if
        local.get $sum
        local.get $ptr
        local.get $i
        i32.const 8
        i32.mul
        i32.add
        f64.load
        f64.add
        local.set $sum
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end)
    local.get $sum))
'

session <- wt_app(array_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_prepare() |>
  wt_new_session()
xs <- session |> wt_array_write(c(1.5, 2.25, 3.25), dtype = "f64")
as.data.frame(list(values = I(list(wt_as_array(xs))), sum = session |> wt_call("sum_f64", xs$ptr, xs$length)))
#>         values sum
#> 1 1.5, 2.2....   7
```

## Sandbox REPL, including a webR guest

Wasmtime does not provide a generic REPL. Rwasmtime exposes a REPL-like
API only as a guest protocol boundary:

1.  a component export such as
    `eval: func(code: string) -> repl-result`,
2.  a core module memory ABI such as `alloc`, `eval(ptr, len)`, result
    pointer/length exports, and optional
    stdout/stderr/error/status/completion exports,
3.  a stdio command loop, or
4.  a callback-backed request/reply channel.

A future webR-in-Wasmtime sandbox should be represented as a persistent
webR guest adapter, not as host R evaluation.

``` r
webr <- wt_webr_repl(
  source = "webr.component.wasm",
  runtime = rt,
  wasi = wt_wasi() |>
    wt_wasi_stdio(stdin = "empty", stdout = "capture", stderr = "capture"),
  limits = wt_limits() |>
    wt_limit_memory("1GiB") |>
    wt_limit_wall_time(ms = 10000),
  callbacks = wt_callbacks() |>
    wt_add_callback(
      name = "rwasmtime:host/repl.display",
      fun = function(x) str(x),
      params = list(x = "string"),
      results = NULL
    ),
  protocol = "component",
  eval_export = "webr:host/repl.eval"
)

webr |> wt_repl_info()
#> <WtReplInfo> protocol=component guest=webR backend=pending open=TRUE inputs=0
#>   eval_export: webr:host/repl.eval

webr |> wt_repl_eval("1 + 1")
#> Error:
#> ! wt_repl_eval protocol=component is not implemented for this Rwasmtime build or API path
```

Native backend builds already support the core-memory protocol for
guests that choose that ABI: an exported memory, an input allocation
path, an `eval(ptr, len)` export, and explicit
result/stdout/stderr/error/status exports.

A useful real target is Simon Willison’s
[`micropython-wasm`](https://github.com/simonw/micropython-wasm), which
ships a WASI MicroPython artifact for Wasmtime. It validates the same
high-level shape: run MicroPython as a guest, pass source through
explicit host/guest channels, capture stdout/stderr, and keep state in a
persistent session. Current Rwasmtime cannot run that artifact yet
because it uses Wasm exception handling and imports custom
`micropython_wasm.host_call` / `host_result_cap` functions in addition
to WASI. That makes it an honest next integration test once
`exceptions = TRUE` and typed custom host imports are implemented.

``` r
core_repl_wat <- '
(module
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 1024))
  (global $result_len (mut i32) (i32.const 0))
  (func (export "alloc") (param $len i32) (result i32)
    (local $ptr i32)
    global.get $heap
    local.set $ptr
    global.get $heap
    local.get $len
    i32.add
    global.set $heap
    local.get $ptr)
  (func (export "repl_eval") (param i32 i32) (result i32)
    (i32.store8 (i32.const 64) (i32.const 111))
    (i32.store8 (i32.const 65) (i32.const 107))
    (global.set $result_len (i32.const 2))
    i32.const 0)
  (func (export "result_ptr") (result i32) i32.const 64)
  (func (export "result_len") (result i32) global.get $result_len))
'

repl <- wt_app(core_repl_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_prepare() |>
  wt_repl(
    protocol = "core",
    eval_export = "repl_eval",
    memory = "memory",
    alloc_export = "alloc",
    result_ptr_export = "result_ptr",
    result_len_export = "result_len"
  )

(repl |> wt_repl_eval("1 + 1"))$value
#> [1] "ok"
```

The package includes `protocol = "mock"` only for API-shape tests. It is
not a sandbox and must not become the webR implementation.

## Development

Use the repository Makefile. It mirrors the Ropendal-style workflow:
generate wrappers/docs first, install source, then test or check. The
target list below is rendered from `make help`, not copied by hand.

``` bash
make --no-print-directory help
Common development targets:
  make rd          regenerate savvy wrappers, roxygen docs, and NAMESPACE
  make rdm         render README.Rmd to README.md with native backend examples
  make authors     regenerate inst/AUTHORS and inst/LICENCE.note from Cargo metadata
  make dev-install install current source with configure/preclean
  make test-api-surface verify intended R API exports/docs
  make test-fast   run non-network tinytest
  make test-rust   verify R-free Rust core boundary, then run Rust unit tests
  make test-rust-backend run feature-gated real Wasmtime/Cranelift smoke tests
  make test-rust-wasi run feature-gated real WASIp1 smoke tests
  make test-c-api  run C API symbol, R-free boundary, Rtinycc header, and installed-symbol checks
  make test-c-api-rust-backend install with Rust/Wasmtime backend and exercise real runtime_build/call_core
  make test-r-runtime-rust-backend verify wt_build_runtime returns a native Savvy runtime handle
  make test-r-aot-rust-backend verify core AOT save/load roundtrip and metadata gate
  make test-r-call-rust-backend verify wt_call executes a real core Wasm function
  make test-r-exec-rust-backend verify wt_exec keeps persistent side-effect calls pipeable
  make test-r-low-level-rust-backend verify wt_compile/wt_instantiate native core artifacts
  make test-r-memory-rust-backend verify persistent sessions, typed memory, and guest arrays
  make test-r-repl-rust-backend verify core-memory persistent REPL protocol transport
  make test-r-wasi-rust-backend verify prepared and low-level WASIp1 execution
  make test-r-callbacks-rust-backend verify core Wasm imports call R callbacks
  make test-r-native-rust-backend install once and run all native backend tinytests
  make test-webr   verify wasm/webR configure selects generated-symbol stubs
  make test        install then run tinytest
  make check       build and run R CMD check --as-cran --no-manual
  make clean       remove local build products
```

The development tests include normal installed-package tinytests, Rust
unit tests, C API boundary checks, and native-backend tests for the
runtime paths shown above.
