source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

callback_wat <- '
(module
  (import "r" "add_one" (func $add_one (param i32) (result i32)))
  (func (export "run") (param i32) (result i32)
    local.get 0
    call $add_one
    i32.const 40
    i32.add))
'

multi_result_wat <- '
(module
  (import "r" "split" (func $split (param i32) (result i32 i32)))
  (func (export "sum_split") (param i32) (result i32)
    local.get 0
    call $split
    i32.add))
'

duplicate_import_wat <- '
(module
  (import "r" "add_one" (func $a (param i32) (result i32)))
  (import "r" "add_one" (func $b (param i32) (result i32)))
  (func (export "run") (param i32) (result i32)
    local.get 0
    call $a
    call $b))
'

wasi_import_wat <- '
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "_start")))
'

rt <- rwasmtime_backend_runtime()
if (!identical(rt$backend, "native")) exit_file("native Rust/Wasmtime backend is not available in this install")

calls <- new.env(parent = emptyenv())
calls$n <- 0L
callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "add_one",
    fun = function(x) {
      calls$n <- calls$n + 1L
      as.integer(x + 1L)
    },
    params = "i32",
    results = "i32",
    abi = "core"
  )

artifact <- rt |> wt_compile(callback_wat, kind = "module")
instance <- artifact |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(callbacks)
  )
expect_class(instance, "WtInstance")
expect_equal(instance$backend, "native")
expect_equal(wt_call(instance, "run", 1L), 42L)
expect_equal(calls$n, 1L)
expect_equal(wt_call(instance, "run", 2L), 43L)
expect_equal(calls$n, 2L)
missing_export <- tryCatch(wt_call(instance, "missing", 1L), error = identity)
expect_true(inherits(missing_export, "error"))
expect_false(inherits(missing_export, "rwasmtime_callback_error"))
expect_false(inherits(missing_export, "rwasmtime_trap"))

prepared <- wt_app(callback_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_callbacks(callbacks) |>
  wt_prepare()
expect_equal(wt_call(prepared, "run", 1L), 42L)
expect_equal(calls$n, 3L)

session <- prepared |> wt_new_session()
expect_equal(session$backend, "native")
expect_equal(wt_call(session, "run", 1L), 42L)
expect_equal(wt_call(session, "run", 2L), 43L)
expect_equal(calls$n, 5L)

multi_callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "split",
    fun = function(x) list(as.integer(x), as.integer(x + 1L)),
    params = "i32",
    results = c("i32", "i32"),
    abi = "core"
  )
multi <- rt |>
  wt_compile(multi_result_wat, kind = "module") |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(multi_callbacks)
  )
expect_equal(wt_call(multi, "sum_split", 20L), 41L)

duplicate <- rt |>
  wt_compile(duplicate_import_wat, kind = "module") |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(callbacks)
  )
expect_equal(wt_call(duplicate, "run", 1L), 3L)
expect_equal(calls$n, 7L)

limited_calls <- new.env(parent = emptyenv())
limited_calls$n <- 0L
limited_callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "add_one",
    fun = function(x) {
      limited_calls$n <- limited_calls$n + 1L
      as.integer(x + 1L)
    },
    params = "i32",
    results = "i32",
    abi = "core",
    policy = wt_callback_policy(max_calls = 1L)
  )
limited_instance <- artifact |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(limited_callbacks)
  )
expect_equal(wt_call(limited_instance, "run", 1L), 42L)
expect_equal(limited_calls$n, 1L)
err <- expect_error_class(wt_call(limited_instance, "run", 1L), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::add_one` failed", conditionMessage(err), fixed = TRUE))
expect_equal(limited_calls$n, 1L)

limited_session <- wt_app(callback_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_callbacks(limited_callbacks) |>
  wt_prepare() |>
  wt_new_session()
expect_equal(wt_call(limited_session, "run", 1L), 42L)
err <- expect_error_class(wt_call(limited_session, "run", 1L), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::add_one` failed", conditionMessage(err), fixed = TRUE))

limited_prepared <- wt_app(callback_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_callbacks(limited_callbacks) |>
  wt_prepare()
expect_equal(wt_call(limited_prepared, "run", 1L), 42L)
expect_equal(wt_call(limited_prepared, "run", 1L), 42L)

zero_callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "add_one",
    fun = function(x) as.integer(x + 1L),
    params = "i32",
    results = "i32",
    abi = "core",
    policy = wt_callback_policy(max_calls = 0L)
  )
zero_instance <- artifact |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(zero_callbacks)
  )
err <- expect_error_class(wt_call(zero_instance, "run", 1L), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::add_one` failed", conditionMessage(err), fixed = TRUE))

duplicate_limited_callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "add_one",
    fun = function(x) as.integer(x + 1L),
    params = "i32",
    results = "i32",
    abi = "core",
    policy = wt_callback_policy(max_calls = 1L)
  )
duplicate_limited <- rt |>
  wt_compile(duplicate_import_wat, kind = "module") |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(duplicate_limited_callbacks)
  )
err <- expect_error_class(wt_call(duplicate_limited, "run", 1L), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::add_one` failed", conditionMessage(err), fixed = TRUE))

slow_callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "add_one",
    fun = function(x) {
      Sys.sleep(0.01)
      as.integer(x + 1L)
    },
    params = "i32",
    results = "i32",
    abi = "core",
    policy = wt_callback_policy(timeout_ms = 0)
  )
slow_instance <- artifact |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(slow_callbacks)
  )
err <- expect_error_class(wt_call(slow_instance, "run", 1L), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::add_one` failed", conditionMessage(err), fixed = TRUE))

failing_callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "add_one",
    fun = function(x) stop("boom", call. = FALSE),
    params = "i32",
    results = "i32",
    abi = "core"
  )
failing_instance <- artifact |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(failing_callbacks)
  )
err <- expect_error_class(wt_call(failing_instance, "run", 1L), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::add_one` failed", conditionMessage(err), fixed = TRUE))
expect_true(inherits(err$parent, "error"))

failing_prepared <- wt_app(callback_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_callbacks(failing_callbacks) |>
  wt_prepare()
err <- expect_error_class(wt_call(failing_prepared, "run", 1L), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::add_one` failed", conditionMessage(err), fixed = TRUE))
failing_session <- failing_prepared |> wt_new_session()
err <- expect_error_class(wt_call(failing_session, "run", 1L), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::add_one` failed", conditionMessage(err), fixed = TRUE))
job <- failing_prepared |> wt_call_async("run", 1L)
status <- wt_poll(job)
expect_true(isTRUE(status$error))
err <- expect_error_class(job |> wt_await(), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::add_one` failed", conditionMessage(err), fixed = TRUE))

bad_result_callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "split",
    fun = function(x) list(as.integer(x)),
    params = "i32",
    results = c("i32", "i32"),
    abi = "core"
  )
bad_result <- rt |>
  wt_compile(multi_result_wat, kind = "module") |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_callbacks(bad_result_callbacks)
  )
err <- expect_error_class(wt_call(bad_result, "sum_split", 20L), "rwasmtime_callback_error")
expect_true(grepl("R callback `r::split` returned", conditionMessage(err), fixed = TRUE))

wasi_callbacks <- wt_wasi() |>
  wt_wasi_stdio(stdout = "capture", stderr = "capture")
wasi_plus_callbacks <- wt_app(wasi_import_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_wasi(wasi_callbacks) |>
  wt_with_callbacks(callbacks) |>
  wt_prepare()
err <- tryCatch(wt_call(wasi_plus_callbacks, "_start"), error = identity)
expect_true(inherits(err, "rwasmtime_not_implemented"))
err <- tryCatch(wt_new_session(wasi_plus_callbacks), error = identity)
expect_true(inherits(err, "rwasmtime_not_implemented"))
job <- wasi_plus_callbacks |> wt_call_async("_start")
status <- wt_poll(job)
expect_class(status, "WtJobStatus")
expect_true(isTRUE(status$error))
err <- tryCatch(job |> wt_await(), error = identity)
expect_true(inherits(err, "rwasmtime_not_implemented"))

err <- tryCatch(
  artifact |> wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_wasi(wasi_callbacks) |> wt_link_callbacks(callbacks)
  ),
  error = identity
)
expect_true(inherits(err, "rwasmtime_not_implemented"))

missing <- tryCatch(
  artifact |> wt_instantiate(store = rt |> wt_store(), linker = rt |> wt_linker()),
  error = identity
)
expect_true(inherits(missing, "error"))
expect_false(inherits(missing, "rwasmtime_callback_error"))
expect_true(grepl("expected 1 imports", conditionMessage(missing), fixed = TRUE) || grepl("missing", conditionMessage(missing), fixed = TRUE))

missing_prepared <- wt_app(callback_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_callbacks(wt_callbacks()) |>
  wt_prepare()
missing <- tryCatch(wt_call(missing_prepared, "run", 1L), error = identity)
expect_true(inherits(missing, "error"))
expect_false(inherits(missing, "rwasmtime_callback_error"))
expect_true(grepl("missing R callback", conditionMessage(missing), fixed = TRUE))
missing_job <- missing_prepared |> wt_call_async("run", 1L)
expect_true(isTRUE(wt_poll(missing_job)$error))
missing <- tryCatch(missing_job |> wt_await(), error = identity)
expect_true(inherits(missing, "error"))
expect_false(inherits(missing, "rwasmtime_callback_error"))
expect_true(grepl("missing R callback", conditionMessage(missing), fixed = TRUE))

extra_callbacks <- callbacks |>
  wt_add_callback(module = "r", name = "not_imported", fun = function() NULL, abi = "core")
err <- tryCatch(
  artifact |> wt_instantiate(store = rt |> wt_store(), linker = rt |> wt_linker() |> wt_link_callbacks(extra_callbacks)),
  error = identity
)
expect_true(inherits(err, "error"))
expect_false(inherits(err, "rwasmtime_callback_error"))
expect_true(grepl("not imported", conditionMessage(err), fixed = TRUE))
extra_prepared <- wt_app(callback_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_callbacks(extra_callbacks) |>
  wt_prepare()
err <- tryCatch(wt_call(extra_prepared, "run", 1L), error = identity)
expect_true(inherits(err, "error"))
expect_false(inherits(err, "rwasmtime_callback_error"))
expect_true(grepl("not imported", conditionMessage(err), fixed = TRUE))
extra_job <- extra_prepared |> wt_call_async("run", 1L)
expect_true(isTRUE(wt_poll(extra_job)$error))
err <- tryCatch(extra_job |> wt_await(), error = identity)
expect_true(inherits(err, "error"))
expect_false(inherits(err, "rwasmtime_callback_error"))
expect_true(grepl("not imported", conditionMessage(err), fixed = TRUE))
