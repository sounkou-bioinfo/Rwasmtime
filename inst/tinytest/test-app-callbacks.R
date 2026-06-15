source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

callbacks <- wt_callbacks() |>
  wt_add_callback(
    name = "rwasmtime:host/callbacks.log",
    fun = function(msg) invisible(NULL),
    params = list(msg = "string"),
    results = NULL
  ) |>
  wt_add_callback(
    module = "r",
    name = "score_f64",
    fun = function(x, y) x + y,
    params = c("f64", "f64"),
    results = "f64",
    abi = "core"
  )

expect_class(callbacks, "WtCallbacks")
expect_equal(length(callbacks$entries), 2L)
callbacks_print <- paste(capture.output(callbacks_returned <- print(callbacks)), collapse = "\n")
expect_identical(callbacks_returned, callbacks)
expect_true(grepl("entries=2", callbacks_print, fixed = TRUE))
expect_true(grepl("rwasmtime:host/callbacks.log", callbacks_print, fixed = TRUE))
expect_true(grepl("r::score_f64", callbacks_print, fixed = TRUE))
policy <- wt_callback_policy(timeout_ms = 10, max_calls = 2, max_depth = 2, reentrant = TRUE)
policy_print <- capture.output(policy_returned <- print(policy))
expect_identical(policy_returned, policy)
expect_true(grepl("timeout_ms=10", policy_print[[1L]], fixed = TRUE))
expect_true(grepl("max_calls=2", policy_print[[1L]], fixed = TRUE))
expect_true(grepl("max_depth=2", policy_print[[1L]], fixed = TRUE))
expect_true(grepl("reentrant=TRUE", policy_print[[1L]], fixed = TRUE))

err <- tryCatch(
  wt_callbacks() |> wt_add_callback(
    name = "bad.component",
    module = "r",
    fun = function() NULL
  ),
  error = identity
)
expect_equal(conditionMessage(err), "component callbacks must not set module")
err <- tryCatch(
  wt_callbacks() |> wt_add_callback(
    name = "score",
    fun = function() NULL,
    abi = "core"
  ),
  error = identity
)
expect_equal(conditionMessage(err), "core callbacks require module")
err <- tryCatch(
  wt_callbacks() |>
    wt_add_callback(name = "dup", fun = function() NULL) |>
    wt_add_callback(name = "dup", fun = function() NULL),
  error = identity
)
expect_equal(conditionMessage(err), "duplicate callback import: dup")
err <- tryCatch(
  wt_callbacks() |> wt_add_callback(
    name = "log",
    fun = function() NULL,
    results = "string",
    policy = wt_callback_policy("fire_and_forget")
  ),
  error = identity
)
expect_equal(conditionMessage(err), "fire-and-forget callbacks must not declare results")
err <- tryCatch(wt_callback_policy(max_depth = 1, reentrant = TRUE), error = identity)
expect_equal(conditionMessage(err), "reentrant callbacks require max_depth of at least 2")

app <- wt_app("plugin.component.wasm") |>
  wt_as_component() |>
  wt_with_runtime(wt_build_runtime(wt_runtime_spec())) |>
  wt_with_callbacks(callbacks) |>
  wt_prepare()

expect_class(app, "WtPreparedApp")

err <- tryCatch(app |> wt_call("run"), error = identity)
expect_true(inherits(err, "rwasmtime_not_implemented"))

job <- app |> wt_call_async("run")
expect_class(job, "WtJob")
status <- wt_poll(job)
expect_class(status, "WtJobStatus")
expect_equal(status$state, "pending")
expect_true(is.null(job$callbacks_drained))
expect_identical(job |> wt_drain_callbacks(max = 7), job)
expect_true(is.null(job$callbacks_drained))
err <- expect_error_class(job |> wt_await(timeout_ms = 0), "rwasmtime_timeout")
expect_equal(err$timeout_ms, 0)
expect_identical(err$job, job)
err <- expect_error_class(job |> wt_await(timeout_ms = 1), "rwasmtime_timeout")
expect_equal(err$timeout_ms, 1)
err <- tryCatch(job |> wt_await(timeout_ms = -1), error = identity)
expect_equal(conditionMessage(err), "timeout_ms must be NULL or a non-negative finite scalar")
err <- tryCatch(job |> wt_await(timeout_ms = c(1, 2)), error = identity)
expect_equal(conditionMessage(err), "timeout_ms must be NULL or a non-negative finite scalar")
err <- tryCatch(job |> wt_await(timeout_ms = Inf), error = identity)
expect_equal(conditionMessage(err), "timeout_ms must be NULL or a non-negative finite scalar")
err <- tryCatch(job |> wt_await(), error = identity)
expect_true(inherits(err, "rwasmtime_not_implemented"))
job <- wt_cancel(job)
status <- wt_poll(job)
expect_class(status, "WtJobStatus")
expect_equal(status$state, "cancelled")
err <- tryCatch(job |> wt_await(), error = identity)
expect_equal(conditionMessage(err), "job was cancelled")
expect_true(is.null(job$callbacks_drained))
