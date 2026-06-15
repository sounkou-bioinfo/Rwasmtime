source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

core_repl_wat <- '
(module
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 1024))
  (global $result_ptr (mut i32) (i32.const 4096))
  (global $result_len (mut i32) (i32.const 0))
  (global $stdout_ptr (mut i32) (i32.const 4100))
  (global $stdout_len (mut i32) (i32.const 0))
  (global $stderr_ptr (mut i32) (i32.const 4110))
  (global $stderr_len (mut i32) (i32.const 0))
  (global $error_ptr (mut i32) (i32.const 4120))
  (global $error_len (mut i32) (i32.const 0))
  (global $status (mut i32) (i32.const 0))
  (global $complete (mut i32) (i32.const 1))
  (global $count (mut i32) (i32.const 0))
  (func (export "alloc") (param $len i32) (result i32)
    (local $ptr i32)
    global.get $heap
    local.set $ptr
    global.get $heap
    local.get $len
    i32.add
    i32.const 16
    i32.add
    global.set $heap
    local.get $ptr)
  (func (export "repl_eval") (param $ptr i32) (param $len i32) (result i32)
    (global.set $count (i32.add (global.get $count) (i32.const 1)))
    (i32.store8 (i32.const 4096) (i32.add (i32.const 48) (global.get $count)))
    (i32.store8 (i32.const 4097) (i32.const 58))
    (if (i32.gt_u (local.get $len) (i32.const 0))
      (then
        (i32.store8 (i32.const 4098) (i32.load8_u (local.get $ptr))))
      (else
        (i32.store8 (i32.const 4098) (i32.const 63))))
    (i32.store8 (i32.const 4100) (i32.const 111))
    (i32.store8 (i32.const 4101) (i32.const 117))
    (i32.store8 (i32.const 4102) (i32.const 116))
    (i32.store8 (i32.const 4110) (i32.const 119))
    (i32.store8 (i32.const 4111) (i32.const 114))
    (i32.store8 (i32.const 4112) (i32.const 110))
    (global.set $result_ptr (i32.const 4096))
    (global.set $result_len (i32.const 3))
    (global.set $stdout_ptr (i32.const 4100))
    (global.set $stdout_len (i32.const 3))
    (global.set $stderr_ptr (i32.const 4110))
    (global.set $stderr_len (i32.const 3))
    (global.set $error_ptr (i32.const 4120))
    (global.set $error_len (i32.const 0))
    (global.set $status (i32.const 0))
    (global.set $complete (i32.const 1))
    i32.const 0)
  (func (export "result_ptr") (result i32) global.get $result_ptr)
  (func (export "result_len") (result i32) global.get $result_len)
  (func (export "stdout_ptr") (result i32) global.get $stdout_ptr)
  (func (export "stdout_len") (result i32) global.get $stdout_len)
  (func (export "stderr_ptr") (result i32) global.get $stderr_ptr)
  (func (export "stderr_len") (result i32) global.get $stderr_len)
  (func (export "error_ptr") (result i32) global.get $error_ptr)
  (func (export "error_len") (result i32) global.get $error_len)
  (func (export "status") (result i32) global.get $status)
  (func (export "complete") (result i32) global.get $complete))
'

rt <- rwasmtime_backend_runtime()
if (!identical(rt$backend, "native")) exit_file("native Rust/Wasmtime backend is not available in this install")
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
    result_len_export = "result_len",
    stdout_ptr_export = "stdout_ptr",
    stdout_len_export = "stdout_len",
    stderr_ptr_export = "stderr_ptr",
    stderr_len_export = "stderr_len",
    error_ptr_export = "error_ptr",
    error_len_export = "error_len",
    status_export = "status",
    complete_export = "complete"
  )

info <- wt_repl_info(repl)
expect_class(info, "WtReplInfo")
expect_true(grepl("<WtReplInfo>", capture.output(print(info))[[1L]], fixed = TRUE))
expect_equal(info$protocol, "core")
expect_equal(info$backend, "native")

first <- wt_repl_eval(repl, "alpha")
expect_class(first, "WtReplResult")
expect_equal(first$value, "1:a")
expect_equal(first$stdout, "out")
expect_equal(first$stderr, "wrn")
expect_equal(first$error, "")
expect_equal(first$status, 0L)
expect_true(isTRUE(first$complete))

second <- wt_repl_eval(repl, "beta")
expect_equal(second$value, "2:b")
expect_equal(wt_repl_history(repl), c("alpha", "beta"))
expect_equal(wt_repl_read(repl), c("1:a", "2:b"))
