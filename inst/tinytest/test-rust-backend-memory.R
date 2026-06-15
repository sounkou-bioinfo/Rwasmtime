source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

memory_wat <- '
(module
  (memory (export "memory") 1 2)
  (global $heap (mut i32) (i32.const 256))
  (data (i32.const 0) "abc")
  (func (export "load8") (param i32) (result i32)
    local.get 0
    i32.load8_u)
  (func (export "store8") (param i32 i32)
    local.get 0
    local.get 1
    i32.store8)
  (func (export "load_i32") (param i32) (result i32)
    local.get 0
    i32.load)
  (func (export "load_f64") (param i32) (result f64)
    local.get 0
    f64.load)
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
  (func (export "grow_memory") (param i32) (result i32)
    local.get 0
    memory.grow)
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

alloc_grow_wat <- '
(module
  (memory (export "memory") 1 2)
  (func (export "alloc") (param i32) (result i32)
    (drop (memory.grow (i32.const 1)))
    i32.const 256)
  (func (export "free") (param i32 i32)))
'

table_grow_wat <- '
(module
  (table (export "table") 1 2 funcref)
  (func (export "grow_table") (param i32) (result i32)
    ref.null func
    local.get 0
    table.grow))
'

rt <- rwasmtime_backend_runtime()
if (!identical(rt$backend, "native")) exit_file("native Rust/Wasmtime backend is not available in this install")
session <- wt_app(memory_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_prepare() |>
  wt_new_session()
mem <- session |> wt_memory("memory")

expect_equal(wt_memory_size(mem), 1)
expect_equal(rawToChar(wt_memory_read(mem, ptr = 0, length = 3, dtype = "u8")), "abc")
mem <- wt_memory_write(mem, ptr = 1, value = charToRaw("Z"), dtype = "u8")
expect_equal(rawToChar(wt_memory_read(mem, ptr = 0, length = 3, dtype = "u8")), "aZc")

invisible(wt_call(session, "store8", 2L, as.integer(charToRaw("Q"))))
expect_equal(wt_call(session, "load8", 2L), as.integer(charToRaw("Q")))
expect_equal(rawToChar(wt_memory_read(mem, ptr = 0, length = 3, dtype = "u8")), "aZQ")
expect_equal(wt_memory_grow(mem, pages = 1), 1)
expect_equal(wt_memory_size(mem), 2)

mem <- wt_memory_write(mem, ptr = 16, value = c(42, -7, -2147483648), dtype = "i32")
expect_equal(wt_memory_read(mem, ptr = 16, length = 3, dtype = "i32"), c(42, -7, -2147483648))
expect_equal(wt_call(session, "load_i32", 16L), 42L)

mem <- wt_memory_write(mem, ptr = 32, value = c(0, 4294967295), dtype = "u32")
expect_equal(wt_memory_read(mem, ptr = 32, length = 2, dtype = "u32"), c(0, 4294967295))

mem <- wt_memory_write(mem, ptr = 48, value = c(1.5, -2.25), dtype = "f64")
expect_equal(wt_memory_read(mem, ptr = 48, length = 2, dtype = "f64"), c(1.5, -2.25))
expect_equal(wt_call(session, "load_f64", 48L), 1.5)

mem <- wt_memory_write(mem, ptr = 80, value = c(1.5, -2.25), dtype = "f32")
expect_true(isTRUE(all.equal(wt_memory_read(mem, ptr = 80, length = 2, dtype = "f32"), c(1.5, -2.25), tolerance = 1e-6)))

signed64 <- c("-9223372036854775808", "42", "9223372036854775807")
mem <- wt_memory_write(mem, ptr = 96, value = signed64, dtype = "i64")
expect_equal(wt_memory_read(mem, ptr = 96, length = length(signed64), dtype = "i64"), signed64)

unsigned64 <- c("0", "9007199254740993", "18446744073709551615")
mem <- wt_memory_write(mem, ptr = 128, value = unsigned64, dtype = "u64")
expect_equal(wt_memory_read(mem, ptr = 128, length = length(unsigned64), dtype = "u64"), unsigned64)
err <- expect_error_class(wt_memory_write(mem, ptr = 160, value = 9007199254740993, dtype = "u64"), "error")
expect_true(grepl("decimal strings", conditionMessage(err), fixed = TRUE))

array <- wt_array_write(session, c(1.5, 2.25, 3.25), dtype = "f64")
expect_class(array, "WtArray")
expect_true(grepl("<WtArray>", capture.output(print(array))[[1L]], fixed = TRUE))
expect_equal(array$length, 3)
expect_equal(wt_as_array(array), c(1.5, 2.25, 3.25))
expect_equal(wt_call(session, "sum_f64", array$ptr, array$length), 7.0)
invisible(wt_free(array))

one_page_limit <- wt_limits() |> wt_limit_memory(65536)
limited_session <- wt_app(memory_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(one_page_limit) |>
  wt_prepare() |>
  wt_new_session()
limited_mem <- limited_session |> wt_memory("memory")
expect_equal(wt_memory_size(limited_mem), 1)
err <- expect_error_class(wt_memory_grow(limited_mem, pages = 1), "rwasmtime_limit_error")
expect_true(grepl("wt_memory_grow exceeds configured memory limit", conditionMessage(err), fixed = TRUE))
expect_equal(err$limit, 65536)
expect_equal(err$requested, 131072)
expect_equal(wt_memory_size(limited_mem), 1)
err <- expect_error_class(wt_memory_write(limited_mem, ptr = 65535, value = as.raw(c(1, 2)), dtype = "u8"), "rwasmtime_limit_error")
expect_true(grepl("wt_memory_write exceeds configured memory limit", conditionMessage(err), fixed = TRUE))
expect_equal(err$limit, 65536)
expect_equal(err$requested, 65537)

small_limit <- wt_limits() |> wt_limit_memory(260)
small_limited_app <- wt_app(memory_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(small_limit) |>
  wt_prepare()
err <- expect_error_class(wt_new_session(small_limited_app), "rwasmtime_limit_error")
expect_true(grepl("memory limit exceeded", conditionMessage(err), fixed = TRUE))
expect_equal(err$limit, 260)
expect_equal(err$requested, 65536)

guest_grow_session <- wt_app(memory_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(one_page_limit) |>
  wt_prepare() |>
  wt_new_session()
guest_grow_mem <- guest_grow_session |> wt_memory("memory")
err <- expect_error_class(wt_call(guest_grow_session, "grow_memory", 1L), "rwasmtime_limit_error")
expect_true(grepl("memory limit exceeded", conditionMessage(err), fixed = TRUE))
expect_equal(err$limit, 65536)
expect_equal(err$requested, 131072)
expect_equal(wt_memory_size(guest_grow_mem), 1)

grow_alloc_session <- wt_app(alloc_grow_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(one_page_limit) |>
  wt_prepare() |>
  wt_new_session()
grow_alloc_mem <- grow_alloc_session |> wt_memory("memory")
expect_equal(wt_memory_size(grow_alloc_mem), 1)
err <- expect_error_class(wt_array_write(grow_alloc_session, 1.5, dtype = "f64"), "rwasmtime_limit_error")
expect_true(grepl("memory limit exceeded", conditionMessage(err), fixed = TRUE))
expect_equal(wt_memory_size(grow_alloc_mem), 1)
expect_equal(err$limit, 65536)
expect_equal(err$requested, 131072)
expect_identical(wt_memory_read(grow_alloc_mem, ptr = 256, length = 8, dtype = "u8"), raw(8L))

limited_instance <- (rt |> wt_compile(memory_wat, kind = "module")) |>
  wt_instantiate(store = rt |> wt_store(limits = one_page_limit), linker = rt |> wt_linker())
limited_instance_mem <- limited_instance |> wt_memory("memory")
err <- expect_error_class(wt_memory_grow(limited_instance_mem, pages = 1), "rwasmtime_limit_error")
expect_true(grepl("wt_memory_grow exceeds configured memory limit", conditionMessage(err), fixed = TRUE))
expect_equal(err$limit, 65536)

table_limited_session <- wt_app(table_grow_wat) |>
  wt_as_module() |>
  wt_with_runtime(rt) |>
  wt_with_limits(wt_limits() |> wt_limit_tables(1)) |>
  wt_prepare() |>
  wt_new_session()
err <- expect_error_class(wt_call(table_limited_session, "grow_table", 1L), "rwasmtime_limit_error")
expect_true(grepl("table element limit exceeded", conditionMessage(err), fixed = TRUE))
expect_equal(err$limit, 1)
expect_equal(err$requested, 2)

zero_instance_limit <- wt_limits() |> wt_limit_instances(0)
err <- expect_error_class(
  (rt |> wt_compile(memory_wat, kind = "module")) |>
    wt_instantiate(store = rt |> wt_store(limits = zero_instance_limit), linker = rt |> wt_linker()),
  "rwasmtime_limit_error"
)
expect_true(grepl("resource limit exceeded", conditionMessage(err), fixed = TRUE))
