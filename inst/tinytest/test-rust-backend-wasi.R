source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

echo_wasi_wat <- '
(module
  (import "wasi_snapshot_preview1" "fd_read"
    (func $fd_read (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "_start")
    (i32.store (i32.const 0) (i32.const 32))
    (i32.store (i32.const 4) (i32.const 64))
    (drop (call $fd_read (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 24)))
    (i32.store (i32.const 8) (i32.const 32))
    (i32.store (i32.const 12) (i32.load (i32.const 24)))
    (drop (call $fd_write (i32.const 1) (i32.const 8) (i32.const 1) (i32.const 28)))))
'

stdio_file_wasi_wat <- '
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 32) "out\\n")
  (data (i32.const 40) "err\\n")
  (func (export "_start")
    (i32.store (i32.const 0) (i32.const 32))
    (i32.store (i32.const 4) (i32.const 4))
    (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 24)))
    (i32.store (i32.const 8) (i32.const 40))
    (i32.store (i32.const 12) (i32.const 4))
    (drop (call $fd_write (i32.const 2) (i32.const 8) (i32.const 1) (i32.const 28)))))
'

binary_wasi_wat <- '
(module
  (import "wasi_snapshot_preview1" "fd_write"
    (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (data (i32.const 32) "A\\00B")
  (data (i32.const 40) "C\\00D")
  (func (export "_start")
    (i32.store (i32.const 0) (i32.const 32))
    (i32.store (i32.const 4) (i32.const 3))
    (drop (call $fd_write (i32.const 1) (i32.const 0) (i32.const 1) (i32.const 24)))
    (i32.store (i32.const 8) (i32.const 40))
    (i32.store (i32.const 12) (i32.const 3))
    (drop (call $fd_write (i32.const 2) (i32.const 8) (i32.const 1) (i32.const 28)))))
'

wasi_callback_wat <- '
(module
  (import "wasi_snapshot_preview1" "args_sizes_get"
    (func $args_sizes_get (param i32 i32) (result i32)))
  (import "r" "answer" (func $answer (result i32)))
  (memory (export "memory") 1)
  (func (export "run") (result i32)
    (drop (call $args_sizes_get (i32.const 0) (i32.const 4)))
    (call $answer)))
'

rt <- rwasmtime_backend_runtime()
if (!identical(rt$backend, "native")) exit_file("native Rust/Wasmtime backend is not available in this install")
wasi <- wt_wasi() |>
  wt_wasi_stdio(stdin = "string", input = "hello from R WASI", stdout = "capture", stderr = "capture")

app <- wt_app(echo_wasi_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_wasi(wasi) |>
  wt_prepare()

result <- wt_call(app, "_start")
expect_class(result, "WtWasiResult")
expect_true(grepl("<WtWasiResult>", capture.output(print(result))[[1L]], fixed = TRUE))
expect_equal(result$stdout, "hello from R WASI")
expect_equal(result$stderr, "")

stdin_file <- tempfile()
writeBin(charToRaw("hello from file"), stdin_file, useBytes = TRUE)
file_stdin_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdin = "file", stdin_file = stdin_file, stdout = "capture", stderr = "capture")
file_stdin_result <- wt_app(echo_wasi_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_wasi(file_stdin_wasi) |>
  wt_prepare() |>
  wt_call("_start")
expect_equal(file_stdin_result$stdout, "hello from file")

artifact <- rt |> wt_compile(echo_wasi_wat, kind = "module")
store <- rt |> wt_store()
linker <- rt |> wt_linker() |> wt_link_wasi(wasi)
instance <- artifact |> wt_instantiate(store = store, linker = linker)
expect_class(instance, "WtInstance")
expect_equal(instance$backend, "native")
low_level_result <- wt_call(instance, "_start")
expect_class(low_level_result, "WtWasiResult")
expect_equal(low_level_result$stdout, "hello from R WASI")

low_stdin_file <- tempfile()
writeBin(charToRaw("low file stdin"), low_stdin_file, useBytes = TRUE)
low_file_stdin_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdin = "file", stdin_file = low_stdin_file, stdout = "capture", stderr = "capture")
low_file_stdin_instance <- artifact |>
  wt_instantiate(store = rt |> wt_store(), linker = rt |> wt_linker() |> wt_link_wasi(low_file_stdin_wasi))
low_file_stdin_result <- wt_call(low_file_stdin_instance, "_start")
expect_equal(low_file_stdin_result$stdout, "low file stdin")

callbacks <- wt_callbacks() |>
  wt_add_callback("answer", function() 42L, module = "r", abi = "core")
wasi_callback_artifact <- rt |> wt_compile(wasi_callback_wat, kind = "module")
wasi_callback_instance <- wasi_callback_artifact |>
  wt_instantiate(
    store = rt |> wt_store(),
    linker = rt |> wt_linker() |> wt_link_wasi(wt_wasi() |> wt_wasi_args("guest")) |> wt_link_callbacks(callbacks)
  )
expect_equal(wasi_callback_instance |> wt_call("run"), 42L)

nul_stdin_file <- tempfile()
writeBin(as.raw(c(0x41, 0x00, 0x42)), nul_stdin_file, useBytes = TRUE)
nul_stdin_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdin = "file", stdin_file = nul_stdin_file, stdout = "capture", stderr = "capture")
nul_stdin_app <- wt_app(echo_wasi_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_wasi(nul_stdin_wasi) |>
  wt_prepare()
nul_stdin_result <- wt_call(nul_stdin_app, "_start")
expect_identical(nul_stdin_result$stdout_raw, as.raw(c(0x41, 0x00, 0x42)))
expect_true(is.na(nul_stdin_result$stdout))

low_nul_stdin_instance <- artifact |>
  wt_instantiate(store = rt |> wt_store(), linker = rt |> wt_linker() |> wt_link_wasi(nul_stdin_wasi))
low_nul_stdin_result <- wt_call(low_nul_stdin_instance, "_start")
expect_identical(low_nul_stdin_result$stdout_raw, as.raw(c(0x41, 0x00, 0x42)))

broken_string_wasi <- wt_wasi() |> wt_wasi_stdio(stdin = "string", input = "x", stdout = "capture", stderr = "capture")
broken_string_wasi$input <- NULL
broken_string_app <- wt_app(echo_wasi_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_wasi(broken_string_wasi) |>
  wt_prepare()
err <- expect_error_class(wt_call(broken_string_app, "_start"), "error")
expect_equal(conditionMessage(err), "stdin='string' requires input")

stdout_file <- tempfile()
stderr_file <- tempfile()
file_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdout = "file", stderr = "file", stdout_file = stdout_file, stderr_file = stderr_file)
file_app <- wt_app(stdio_file_wasi_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_wasi(file_wasi) |>
  wt_prepare()
file_result <- wt_call(file_app, "_start")
expect_class(file_result, "WtWasiResult")
expect_equal(readChar(stdout_file, file.info(stdout_file)$size, useBytes = TRUE), "out\n")
expect_equal(readChar(stderr_file, file.info(stderr_file)$size, useBytes = TRUE), "err\n")
expect_equal(file_result$stdout, "out\n")
expect_equal(file_result$stderr, "err\n")
expect_equal(file_result$stdout_file, stdout_file)
expect_equal(file_result$stderr_file, stderr_file)
expect_true(grepl("stdout_file", paste(capture.output(print(file_result)), collapse = "\n"), fixed = TRUE))

low_stdout_file <- tempfile()
low_stderr_file <- tempfile()
low_file_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdout = "file", stderr = "file", stdout_file = low_stdout_file, stderr_file = low_stderr_file)
low_file_instance <- (rt |> wt_compile(stdio_file_wasi_wat, kind = "module")) |>
  wt_instantiate(store = rt |> wt_store(), linker = rt |> wt_linker() |> wt_link_wasi(low_file_wasi))
low_file_result <- wt_call(low_file_instance, "_start")
expect_class(low_file_result, "WtWasiResult")
expect_equal(readChar(low_stdout_file, file.info(low_stdout_file)$size, useBytes = TRUE), "out\n")
expect_equal(readChar(low_stderr_file, file.info(low_stderr_file)$size, useBytes = TRUE), "err\n")
expect_equal(low_file_result$stdout_file, low_stdout_file)
expect_equal(low_file_result$stderr_file, low_stderr_file)

low_mixed_stdout_file <- tempfile()
low_mixed_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdout = "file", stderr = "capture", stdout_file = low_mixed_stdout_file)
low_mixed_instance <- (rt |> wt_compile(stdio_file_wasi_wat, kind = "module")) |>
  wt_instantiate(store = rt |> wt_store(), linker = rt |> wt_linker() |> wt_link_wasi(low_mixed_wasi))
low_mixed_result <- wt_call(low_mixed_instance, "_start")
expect_equal(readChar(low_mixed_stdout_file, file.info(low_mixed_stdout_file)$size, useBytes = TRUE), "out\n")
expect_equal(low_mixed_result$stderr, "err\n")
expect_equal(low_mixed_result$stdout_file, low_mixed_stdout_file)
expect_true(is.null(low_mixed_result$stderr_file))

low_mixed_stderr_file <- tempfile()
low_mixed_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdout = "capture", stderr = "file", stderr_file = low_mixed_stderr_file)
low_mixed_instance <- (rt |> wt_compile(stdio_file_wasi_wat, kind = "module")) |>
  wt_instantiate(store = rt |> wt_store(), linker = rt |> wt_linker() |> wt_link_wasi(low_mixed_wasi))
low_mixed_result <- wt_call(low_mixed_instance, "_start")
expect_equal(low_mixed_result$stdout, "out\n")
expect_equal(readChar(low_mixed_stderr_file, file.info(low_mixed_stderr_file)$size, useBytes = TRUE), "err\n")
expect_true(is.null(low_mixed_result$stdout_file))
expect_equal(low_mixed_result$stderr_file, low_mixed_stderr_file)

mixed_stdout_file <- tempfile()
mixed_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdout = "file", stderr = "capture", stdout_file = mixed_stdout_file)
mixed_result <- wt_app(stdio_file_wasi_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_wasi(mixed_wasi) |>
  wt_prepare() |>
  wt_call("_start")
expect_equal(readChar(mixed_stdout_file, file.info(mixed_stdout_file)$size, useBytes = TRUE), "out\n")
expect_equal(mixed_result$stderr, "err\n")
expect_equal(mixed_result$stdout_file, mixed_stdout_file)
expect_true(is.null(mixed_result$stderr_file))

mixed_stderr_file <- tempfile()
mixed_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdout = "capture", stderr = "file", stderr_file = mixed_stderr_file)
mixed_result <- wt_app(stdio_file_wasi_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_wasi(mixed_wasi) |>
  wt_prepare() |>
  wt_call("_start")
expect_equal(mixed_result$stdout, "out\n")
expect_equal(readChar(mixed_stderr_file, file.info(mixed_stderr_file)$size, useBytes = TRUE), "err\n")
expect_true(is.null(mixed_result$stdout_file))
expect_equal(mixed_result$stderr_file, mixed_stderr_file)

binary_stdout_file <- tempfile()
binary_stderr_file <- tempfile()
binary_wasi <- wt_wasi() |>
  wt_wasi_stdio(stdout = "file", stderr = "file", stdout_file = binary_stdout_file, stderr_file = binary_stderr_file)
binary_result <- wt_app(binary_wasi_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_wasi(binary_wasi) |>
  wt_prepare() |>
  wt_call("_start")
binary_stdout <- as.raw(c(0x41, 0x00, 0x42))
binary_stderr <- as.raw(c(0x43, 0x00, 0x44))
expect_identical(readBin(binary_stdout_file, "raw", n = file.info(binary_stdout_file)$size), binary_stdout)
expect_identical(readBin(binary_stderr_file, "raw", n = file.info(binary_stderr_file)$size), binary_stderr)
expect_identical(binary_result$stdout_raw, binary_stdout)
expect_identical(binary_result$stderr_raw, binary_stderr)
expect_true(is.na(binary_result$stdout))
expect_true(is.na(binary_result$stderr))
expect_true(grepl("<non-text bytes>", paste(capture.output(print(binary_result)), collapse = "\n"), fixed = TRUE))

err <- expect_error_class(wt_call(app, "not_start"), "rwasmtime_not_implemented")
expect_true(grepl("WASI command", conditionMessage(err), fixed = TRUE))
