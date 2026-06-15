source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

rt <- wt_runtime_spec() |>
  wt_with_compiler("cranelift", opt_level = "speed") |>
  wt_enable_features(component_model = TRUE, simd = TRUE, relaxed_simd = FALSE) |>
  wt_with_aot(cache = TRUE) |>
  wt_build_runtime()

expect_class(rt, "WtRuntime")
rt_print <- capture.output(rt_returned <- print(rt))
expect_identical(rt_returned, rt)
expect_true(grepl("<WtRuntime>", rt_print[[1L]], fixed = TRUE))
expect_equal(rt$backend, "pending")
expect_true(is.null(rt$ptr))
expect_equal(rt$spec$compiler$strategy, "cranelift")
expect_true(rt$spec$features$simd)
expect_false(rt$spec$features$relaxed_simd)
expect_false(rt$spec$features$relaxed_simd_deterministic)

relaxed <- wt_runtime_spec() |>
  wt_enable_features(relaxed_simd = TRUE, relaxed_simd_deterministic = TRUE)
expect_true(relaxed$features$relaxed_simd)
expect_true(relaxed$features$relaxed_simd_deterministic)
err <- tryCatch(wt_runtime_spec() |> wt_enable_features(relaxed_simd_deterministic = TRUE), error = identity)
expect_equal(conditionMessage(err), "relaxed_simd_deterministic requires relaxed_simd = TRUE")
err <- tryCatch(wt_runtime_spec() |> wt_enable_features(simd = FALSE, relaxed_simd = TRUE), error = identity)
expect_equal(conditionMessage(err), "relaxed_simd requires simd = TRUE")
err <- tryCatch(wt_runtime_spec() |> wt_enable_features(component_model = FALSE, component_model_async = TRUE), error = identity)
expect_equal(conditionMessage(err), "component_model_async requires component_model = TRUE")
err <- tryCatch(wt_runtime_spec() |> wt_with_compiler("winch", opt_level = "speed"), error = identity)
expect_equal(conditionMessage(err), "winch compiler requires opt_level = 'none'")

wasi <- wt_wasi() |>
  wt_wasi_args("--input", "/data/x.csv") |>
  wt_wasi_env(TZ = "UTC") |>
  wt_wasi_preopen("/data", tempfile()) |>
  wt_wasi_stdio(stdout = "capture", stderr = "capture")

expect_class(wasi, "WtWasi")
expect_equal(length(wasi$args), 2L)
expect_equal(length(wasi$preopens), 1L)
expect_false(wasi$network)
expect_false(wasi$clocks)
expect_false(wasi$random)
wasi_print <- capture.output(wasi_returned <- print(wasi))
expect_identical(wasi_returned, wasi)
expect_true(grepl("stdio: stdin=empty stdout=capture stderr=capture", paste(wasi_print, collapse = "\n"), fixed = TRUE))
expect_true(grepl("/data=>", paste(wasi_print, collapse = "\n"), fixed = TRUE))
expect_true(grepl("(ro)", paste(wasi_print, collapse = "\n"), fixed = TRUE))
expect_true(grepl("ambient: network=FALSE clocks=FALSE random=FALSE", paste(wasi_print, collapse = "\n"), fixed = TRUE))
file_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdin = "file", stdout = "file", stderr = "file", stdin_file = "stdin.txt", stdout_file = "stdout.log", stderr_file = "stderr.log")
file_wasi_print <- paste(capture.output(print(file_wasi)), collapse = "\n")
expect_true(grepl("files: stdin=stdin.txt stdout=stdout.log stderr=stderr.log", file_wasi_print, fixed = TRUE))

err <- tryCatch(
  do.call(wt_wasi_env, c(list(.x = wt_wasi()), setNames(list("value"), "BAD=NAME"))),
  error = identity
)
expect_equal(conditionMessage(err), "WASI env names must not contain '='")
err <- tryCatch(wt_wasi() |> wt_wasi_preopen("relative", tempfile()), error = identity)
expect_equal(conditionMessage(err), "guest must be an absolute WASI path")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdin = "string"), error = identity)
expect_equal(conditionMessage(err), "stdin='string' requires input")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(input = "unused"), error = identity)
expect_equal(conditionMessage(err), "input is only valid with stdin='string'")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(input = NA_character_), error = identity)
expect_equal(conditionMessage(err), "input must be a single non-NA string")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(input = c("a", "b")), error = identity)
expect_equal(conditionMessage(err), "input must be a single non-NA string")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdin_file = "unused.txt"), error = identity)
expect_equal(conditionMessage(err), "stdin_file is only valid with stdin='file'")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdin_file = NA_character_), error = identity)
expect_equal(conditionMessage(err), "stdin_file must be a non-empty string")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdin_file = character()), error = identity)
expect_equal(conditionMessage(err), "stdin_file must be a non-empty string")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdin_file = c("a", "b")), error = identity)
expect_equal(conditionMessage(err), "stdin_file must be a non-empty string")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdin = "file"), error = identity)
expect_equal(conditionMessage(err), "stdin='file' requires stdin_file")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdout_file = "unused.log"), error = identity)
expect_equal(conditionMessage(err), "stdout_file is only valid with stdout='file'")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdout_file = NA_character_), error = identity)
expect_equal(conditionMessage(err), "stdout_file must be a non-empty string")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stderr_file = "unused.log"), error = identity)
expect_equal(conditionMessage(err), "stderr_file is only valid with stderr='file'")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stderr_file = NA_character_), error = identity)
expect_equal(conditionMessage(err), "stderr_file must be a non-empty string")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdout = "file"), error = identity)
expect_equal(conditionMessage(err), "stdout='file' requires stdout_file")
err <- tryCatch(wt_wasi() |> wt_wasi_stdio(stdout = "file", stderr = "file", stdout_file = "same.log", stderr_file = "same.log"), error = identity)
expect_equal(conditionMessage(err), "stdout_file and stderr_file must be different until native streaming file sinks are implemented")

with_file <- wt_app("x.wasm") |>
  wt_with_wasi(stdin = "file", stdin_file = "input.txt")
expect_equal(with_file$wasi$stdin, "file")
expect_equal(with_file$wasi$stdin_file, "input.txt")
preconfigured <- wt_wasi() |>
  wt_wasi_stdio(stdout = "inherit", stderr = "discard")
partial <- wt_app("x.wasm") |>
  wt_with_wasi(wasi = preconfigured, stdin = "string", input = "hi")
expect_equal(partial$wasi$stdin, "string")
expect_equal(partial$wasi$input, "hi")
expect_equal(partial$wasi$stdout, "inherit")
expect_equal(partial$wasi$stderr, "discard")
file_configured <- wt_wasi() |>
  wt_wasi_stdio(stdin = "file", stdout = "file", stderr = "file", stdin_file = "in-a.txt", stdout_file = "out-a.log", stderr_file = "err-a.log")
file_updated <- wt_app("x.wasm") |>
  wt_with_wasi(wasi = file_configured, stdin_file = "in-b.txt", stdout_file = "out-b.log", stderr_file = "err-b.log")
expect_equal(file_updated$wasi$stdin, "file")
expect_equal(file_updated$wasi$stdin_file, "in-b.txt")
expect_equal(file_updated$wasi$stdout, "file")
expect_equal(file_updated$wasi$stdout_file, "out-b.log")
expect_equal(file_updated$wasi$stderr, "file")
expect_equal(file_updated$wasi$stderr_file, "err-b.log")
one_sided_file_update <- wt_app("x.wasm") |>
  wt_with_wasi(wasi = file_configured, stdout_file = "out-c.log")
expect_equal(one_sided_file_update$wasi$stdin, "file")
expect_equal(one_sided_file_update$wasi$stdin_file, "in-a.txt")
expect_equal(one_sided_file_update$wasi$stdout, "file")
expect_equal(one_sided_file_update$wasi$stdout_file, "out-c.log")
expect_equal(one_sided_file_update$wasi$stderr, "file")
expect_equal(one_sided_file_update$wasi$stderr_file, "err-a.log")
err <- tryCatch(wt_app("x.wasm") |> wt_with_wasi(input = "unused"), error = identity)
expect_equal(conditionMessage(err), "input is only valid with stdin='string'")
err <- tryCatch(wt_app("x.wasm") |> wt_with_wasi(stdin_file = "input.txt"), error = identity)
expect_equal(conditionMessage(err), "stdin_file is only valid with stdin='file'")
err <- tryCatch(wt_app("x.wasm") |> wt_with_wasi(unknown = TRUE), error = identity)
expect_equal(conditionMessage(err), "unsupported wt_with_wasi argument(s): unknown")

limits <- wt_limits() |>
  wt_limit_memory("1MiB") |>
  wt_limit_wall_time(1000) |>
  wt_limit_callbacks(max_calls = 10, timeout_ms = 100)

expect_class(limits, "WtLimits")
expect_equal(limits$memory_bytes, 1024^2)
expect_equal(limits$wall_time_ms, 1000L)
limits_print <- paste(capture.output(limits_returned <- print(limits)), collapse = "\n")
expect_identical(limits_returned, limits)
expect_true(grepl("memory=1048576", limits_print, fixed = TRUE))
expect_true(grepl("wall_time_ms=1000", limits_print, fixed = TRUE))
expect_true(grepl("callbacks: max_calls=10 timeout_ms=100 max_depth=1 reentrant=FALSE", limits_print, fixed = TRUE))

app_with_dot_limits <- wt_app("plugin.wasm") |>
  wt_with_limits(memory_bytes = "2MiB", table_elements = 2, instances = 3, fuel = 4, wall_time_ms = 5)
expect_equal(app_with_dot_limits$limits$memory_bytes, 2 * 1024^2)
expect_equal(app_with_dot_limits$limits$table_elements, 2)
expect_equal(app_with_dot_limits$limits$instances, 3)
expect_equal(app_with_dot_limits$limits$fuel, 4)
expect_equal(app_with_dot_limits$limits$wall_time_ms, 5)
err <- tryCatch(wt_app("plugin.wasm") |> wt_with_limits(unknown = 1), error = identity)
expect_equal(conditionMessage(err), "unsupported wt_with_limits argument(s): unknown")
err <- tryCatch(wt_app("plugin.wasm") |> wt_with_limits(NULL, 1), error = identity)
expect_equal(conditionMessage(err), "unsupported wt_with_limits argument(s): <unnamed>")

zero_limits <- wt_limits() |>
  wt_limit_tables(0) |>
  wt_limit_instances(0) |>
  wt_limit_fuel(0) |>
  wt_limit_wall_time(0) |>
  wt_limit_callbacks(max_calls = 0, timeout_ms = 0, max_depth = 1)
expect_equal(zero_limits$table_elements, 0L)
expect_equal(zero_limits$instances, 0L)
expect_equal(zero_limits$fuel, 0)
expect_equal(zero_limits$wall_time_ms, 0L)
expect_equal(zero_limits$max_callback_calls, 0)
expect_equal(zero_limits$callback_timeout_ms, 0)
expect_false(is.null(zero_limits$max_callback_calls))
large_wall_limits <- wt_limits() |> wt_limit_wall_time(2147483648)
expect_equal(large_wall_limits$wall_time_ms, 2147483648)

err <- tryCatch(wt_limits() |> wt_limit_tables(-1), error = identity)
expect_equal(conditionMessage(err), "table element limit must be a non-negative whole finite scalar")
err <- tryCatch(wt_limits() |> wt_limit_tables(1.5), error = identity)
expect_equal(conditionMessage(err), "table element limit must be a non-negative whole finite scalar")
err <- tryCatch(wt_limits() |> wt_limit_instances(1.5), error = identity)
expect_equal(conditionMessage(err), "instance limit must be a non-negative whole finite scalar")
err <- tryCatch(wt_limits() |> wt_limit_fuel(Inf), error = identity)
expect_equal(conditionMessage(err), "fuel limit must be a non-negative whole finite scalar")
err <- tryCatch(wt_limits() |> wt_limit_fuel(1.5), error = identity)
expect_equal(conditionMessage(err), "fuel limit must be a non-negative whole finite scalar")
err <- tryCatch(wt_limits() |> wt_limit_wall_time(1.5), error = identity)
expect_equal(conditionMessage(err), "wall time limit must be a non-negative whole finite scalar")
err <- tryCatch(wt_limits() |> wt_limit_callbacks(max_calls = 1.5), error = identity)
expect_equal(conditionMessage(err), "callback call limit must be NULL or a non-negative whole finite scalar")
err <- tryCatch(wt_limits() |> wt_limit_callbacks(max_depth = 0), error = identity)
expect_equal(conditionMessage(err), "callback max_depth must be at least 1")
err <- tryCatch(wt_limits() |> wt_limit_callbacks(reentrant = TRUE, max_depth = 1), error = identity)
expect_equal(conditionMessage(err), "reentrant callbacks require max_depth of at least 2")
err <- tryCatch(wt_callback_policy(timeout_ms = -1), error = identity)
expect_equal(conditionMessage(err), "callback timeout must be NULL or a non-negative finite scalar")
err <- tryCatch(wt_callback_policy(max_calls = 1.5), error = identity)
expect_equal(conditionMessage(err), "callback call limit must be NULL or a non-negative whole finite scalar")
