source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

exec_wat <- '
(module
  (memory (export "memory") 1)
  (func (export "store8") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8)
  (func (export "load8") (param i32) (result i32)
    local.get 0
    i32.load8_u))
'

rt <- rwasmtime_backend_runtime()
if (!identical(rt$backend, "native")) exit_file("native Rust/Wasmtime backend is not available in this install")
session <- wt_app(exec_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_prepare() |>
  wt_new_session()

returned <- session |>
  wt_exec("store8", 0L, 65L) |>
  wt_exec("store8", 1L, 66L)

expect_class(returned, "WtSession")
expect_equal(wt_call(returned, "load8", 0L), 65L)
expect_equal(wt_call(returned, "load8", 1L), 66L)

mem <- returned |> wt_memory("memory")
expect_equal(wt_memory_read(mem, ptr = 0, length = 2, dtype = "u8"), as.raw(c(65L, 66L)))
