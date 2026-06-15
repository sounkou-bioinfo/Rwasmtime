source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

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

rt <- rwasmtime_backend_runtime()
if (!identical(rt$backend, "native")) exit_file("native Rust/Wasmtime backend is not available in this install")
artifact <- rt |> wt_compile(memory_wat, kind = "module")
expect_class(artifact, "WtArtifact")
expect_equal(artifact$backend, "native")
expect_true(wt_artifact_compatible(artifact, rt))

imports <- artifact |> wt_imports()
exports <- artifact |> wt_exports()
bindings <- artifact |> wt_bindings()
expect_equal(length(imports), 0L)
expect_true(inherits(bindings, "WtBindings"))
expect_equal(length(bindings$exports), length(exports))
load8 <- Filter(function(item) identical(item$name, "load8"), exports)[[1L]]
expect_class(load8, "WtCoreItem")
expect_equal(load8$kind, "function")
expect_equal(load8$params, "i32")
expect_equal(load8$results, "i32")
expect_equal(load8$signature, "(i32) -> (i32)")
memory_export <- Filter(function(item) identical(item$name, "memory"), exports)[[1L]]
expect_equal(memory_export$kind, "memory")
expect_equal(memory_export$minimum, "1")
expect_null(memory_export$maximum)
expect_false(memory_export$shared)
expect_false(memory_export$memory64)
expect_true(grepl("<WtCoreItem>", capture.output(print(load8))[[1L]], fixed = TRUE))
expect_true(grepl("<WtBindings>", capture.output(print(bindings))[[1L]], fixed = TRUE))

store <- rt |> wt_store()
linker <- rt |> wt_linker()

add_wasm_raw <- as.raw(c(
  0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x0c, 0x02, 0x60,
  0x02, 0x7f, 0x7f, 0x01, 0x7f, 0x60, 0x01, 0x7f, 0x01, 0x7f, 0x03, 0x03,
  0x02, 0x00, 0x01, 0x07, 0x0e, 0x02, 0x03, 0x61, 0x64, 0x64, 0x00, 0x00,
  0x04, 0x73, 0x70, 0x69, 0x6e, 0x00, 0x01, 0x0a, 0x28, 0x02, 0x07, 0x00,
  0x20, 0x00, 0x20, 0x01, 0x6a, 0x0b, 0x1e, 0x01, 0x01, 0x7f, 0x20, 0x00,
  0x21, 0x01, 0x02, 0x40, 0x03, 0x40, 0x20, 0x01, 0x45, 0x0d, 0x01, 0x20,
  0x01, 0x41, 0x01, 0x6b, 0x21, 0x01, 0x0c, 0x00, 0x0b, 0x0b, 0x20, 0x00,
  0x0b
))
raw_artifact <- rt |> wt_compile(add_wasm_raw, kind = "module")
raw_instance <- raw_artifact |> wt_instantiate(store = store, linker = linker)
expect_equal(raw_instance |> wt_call("add", 20L, 22L), 42L)
raw_path <- tempfile(fileext = ".wasm")
writeBin(add_wasm_raw, raw_path, useBytes = TRUE)
file_artifact <- rt |> wt_compile(raw_path, kind = "module")
file_instance <- file_artifact |> wt_instantiate(store = store, linker = linker)
expect_equal(file_instance |> wt_call("add", 20L, 22L), 42L)

compile_err <- expect_error_class(rt |> wt_compile("(module BAD", kind = "module"), "rwasmtime_compile_error")
expect_false(inherits(compile_err, "rwasmtime_unsupported_feature"))

err <- tryCatch(
  wt_runtime_spec() |>
    wt_enable_features(exceptions = TRUE) |>
    wt_build_runtime(),
  error = identity
)
expect_true(inherits(err, "rwasmtime_unsupported_feature"))
expect_true(grepl("wasm exceptions", conditionMessage(err), fixed = TRUE))

rt2 <- wt_runtime_spec() |> wt_build_runtime()
err <- tryCatch(artifact |> wt_instantiate(store = rt2 |> wt_store(), linker = linker), error = identity)
expect_true(inherits(err, "error"))
expect_true(grepl("store must use the same runtime", conditionMessage(err), fixed = TRUE))
err <- tryCatch(artifact |> wt_instantiate(store = store, linker = rt2 |> wt_linker()), error = identity)
expect_true(inherits(err, "error"))
expect_true(grepl("linker must use the same runtime", conditionMessage(err), fixed = TRUE))
instance_a <- artifact |> wt_instantiate(store = store, linker = linker)
instance_b <- artifact |> wt_instantiate(store = store, linker = linker)
expect_equal(instance_a$backend, "native")

expect_equal(wt_call(instance_a, "load8", 0L), as.integer(charToRaw("a")))
instance_a <- instance_a |> wt_exec("store8", 0L, as.integer(charToRaw("Z")))
expect_equal(wt_call(instance_a, "load8", 0L), as.integer(charToRaw("Z")))
expect_equal(wt_call(instance_b, "load8", 0L), as.integer(charToRaw("a")))

mem <- instance_a |> wt_memory("memory")
expect_equal(rawToChar(wt_memory_read(mem, ptr = 0, length = 3, dtype = "u8")), "Zbc")
