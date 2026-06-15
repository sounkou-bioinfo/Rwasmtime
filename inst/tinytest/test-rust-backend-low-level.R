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

store <- rt |> wt_store()
linker <- rt |> wt_linker()
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
