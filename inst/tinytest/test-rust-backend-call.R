source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

add_wat <- '
(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add)
  (func (export "mix") (param i64 f64) (result f64)
    local.get 0
    f64.convert_i64_s
    local.get 1
    f64.add)
  (func (export "id_i64") (param i64) (result i64)
    local.get 0)
  (func (export "id_v128") (param v128) (result v128)
    local.get 0)
  (func (export "null_funcref") (result funcref)
    ref.null func)
  (func (export "is_null_funcref") (param funcref) (result i32)
    local.get 0
    ref.is_null))
'

trap_wat <- '
(module
  (func (export "boom")
    unreachable))
'

limit_wat <- '
(module
  (func (export "count_to") (param $n i32) (result i32)
    (local $i i32)
    (loop $loop
      local.get $i
      local.get $n
      i32.lt_s
      if
        local.get $i
        i32.const 1
        i32.add
        local.set $i
        br $loop
      end)
    local.get $i))
'

start_limit_wat <- '
(module
  (global $i (mut i32) (i32.const 0))
  (func $start
    (loop $loop
      global.get $i
      i32.const 10000000
      i32.lt_s
      if
        global.get $i
        i32.const 1
        i32.add
        global.set $i
        br $loop
      end))
  (start $start)
  (func (export "done") (result i32)
    global.get $i))
'

rt <- rwasmtime_backend_runtime()
if (!identical(rt$backend, "native")) exit_file("native Rust/Wasmtime backend is not available in this install")
app <- wt_app(add_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_prepare()

expect_equal(app$backend, "native")
expect_class(app$artifact, "WtArtifact")
expect_equal(app$artifact$backend, "native")
app_print <- capture.output(print(app))[[1L]]
expect_true(grepl("backend=native", app_print, fixed = TRUE))
expect_true(grepl("artifact=TRUE", app_print, fixed = TRUE))
expect_equal(wt_call(app, "add", 20L, 22L), 42L)

compiled <- rt |> wt_compile(add_wat, kind = "module")
from_artifact <- wt_app(compiled) |>
  wt_prepare()
expect_class(from_artifact$artifact, "WtArtifact")
expect_equal(from_artifact$backend, "native")
expect_equal(wt_call(from_artifact, "add", 20L, 22L), 42L)
from_artifact_session <- from_artifact |> wt_new_session()
expect_equal(wt_call(from_artifact_session, "add", 20L, 22L), 42L)
from_artifact_job <- from_artifact |> wt_call_async("add", 20L, 22L)
expect_true(isTRUE(wt_poll(from_artifact_job)$done))
expect_equal(from_artifact_job |> wt_await(), 42L)
from_artifact_session_job <- from_artifact_session |> wt_call_async("add", 20L, 22L)
expect_true(isTRUE(wt_poll(from_artifact_session_job)$done))
expect_equal(from_artifact_session_job |> wt_await(), 42L)
job <- app |> wt_call_async("add", 20L, 22L)
expect_class(job, "WtJob")
status <- wt_poll(job)
expect_class(status, "WtJobStatus")
expect_true(isTRUE(status$done))
expect_false(isTRUE(status$error))
expect_equal(job |> wt_await(), 42L)
expect_equal(job |> wt_await(timeout_ms = 0), 42L)
expect_equal(wt_result(job), 42L)
expect_equal(wt_call(app, "mix", 40L, 2.5), 42.5)
expect_equal(wt_call(app, "id_i64", "9223372036854775807"), "9223372036854775807")

err <- expect_error_class(wt_call(app, "id_i64", 9007199254740993), "error")
expect_true(grepl("decimal string", conditionMessage(err), fixed = TRUE))

v128 <- as.raw(0:15)
expect_equal(wt_call(app, "id_v128", v128), v128)

null_ref <- wt_call(app, "null_funcref")
expect_true(is.list(null_ref))
expect_equal(null_ref$type, "funcref")
expect_true(isTRUE(null_ref$is_null))
expect_equal(wt_call(app, "is_null_funcref", NULL), 1L)

err <- expect_error_class(wt_call(app, "missing", 1L, 2L), "error")
expect_false(inherits(err, "rwasmtime_trap"))
expect_true(grepl("missing", conditionMessage(err), fixed = TRUE))

trap_app <- wt_app(trap_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_prepare()
err <- expect_error_class(wt_call(trap_app, "boom"), "rwasmtime_trap")
expect_true(grepl("Wasm call `boom` trapped or failed", conditionMessage(err), fixed = TRUE))
expect_true(inherits(err$parent, "error"))
trap_job <- trap_app |> wt_call_async("boom")
expect_true(isTRUE(wt_poll(trap_job)$error))
err <- expect_error_class(trap_job |> wt_await(), "rwasmtime_trap")
expect_true(grepl("Wasm call `boom` trapped or failed", conditionMessage(err), fixed = TRUE))
trap_instance <- (rt |> wt_compile(trap_wat, kind = "module")) |>
  wt_instantiate(store = rt |> wt_store(), linker = rt |> wt_linker())
err <- expect_error_class(wt_call(trap_instance, "boom"), "rwasmtime_trap")
expect_true(grepl("Wasm call `boom` trapped or failed", conditionMessage(err), fixed = TRUE))

fuel_limits <- wt_limits() |> wt_limit_fuel(0)
fuel_app <- wt_app(add_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(fuel_limits) |>
  wt_prepare()
err <- expect_error_class(wt_call(fuel_app, "add", 1L, 2L), "rwasmtime_limit_error")
expect_true(grepl("all fuel consumed", conditionMessage(err), fixed = TRUE))
expect_true(inherits(err$parent, "error"))
fuel_session <- fuel_app |> wt_new_session()
err <- expect_error_class(wt_call(fuel_session, "add", 1L, 2L), "rwasmtime_limit_error")
expect_true(grepl("all fuel consumed", conditionMessage(err), fixed = TRUE))
fuel_instance <- compiled |>
  wt_instantiate(store = rt |> wt_store(limits = fuel_limits), linker = rt |> wt_linker())
err <- expect_error_class(wt_call(fuel_instance, "add", 1L, 2L), "rwasmtime_limit_error")
expect_true(grepl("all fuel consumed", conditionMessage(err), fixed = TRUE))

wall_app <- wt_app(limit_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(wt_limits() |> wt_limit_wall_time(0)) |>
  wt_prepare()
err <- expect_error_class(wt_call(wall_app, "count_to", 10000000L), "rwasmtime_limit_error")
expect_true(grepl("wall time limit exceeded", conditionMessage(err), fixed = TRUE))

start_fuel_app <- wt_app(start_limit_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(fuel_limits) |>
  wt_prepare()
err <- expect_error_class(wt_call(start_fuel_app, "done"), "rwasmtime_limit_error")
expect_true(grepl("all fuel consumed", conditionMessage(err), fixed = TRUE))
err <- expect_error_class(start_fuel_app |> wt_new_session(), "rwasmtime_limit_error")
expect_true(grepl("all fuel consumed", conditionMessage(err), fixed = TRUE))
start_fuel_artifact <- rt |> wt_compile(start_limit_wat, kind = "module")
err <- expect_error_class(
  start_fuel_artifact |> wt_instantiate(store = rt |> wt_store(limits = fuel_limits), linker = rt |> wt_linker()),
  "rwasmtime_limit_error"
)
expect_true(grepl("all fuel consumed", conditionMessage(err), fixed = TRUE))

start_wall_app <- wt_app(start_limit_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(wt_limits() |> wt_limit_wall_time(0)) |>
  wt_prepare()
err <- expect_error_class(wt_call(start_wall_app, "done"), "rwasmtime_limit_error")
expect_true(grepl("wall time limit exceeded", conditionMessage(err), fixed = TRUE))
