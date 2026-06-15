source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

rt <- wt_runtime_spec() |>
  wt_with_compiler("cranelift", opt_level = "speed") |>
  wt_build_runtime()

wasi <- wt_wasi() |>
  wt_wasi_args("--mode=test") |>
  wt_wasi_stdio(stdout = "capture", stderr = "capture")

limits <- wt_limits() |>
  wt_limit_memory("128MiB") |>
  wt_limit_wall_time(1000)

callbacks <- wt_callbacks() |>
  wt_add_callback(
    name = "rwasmtime:host/callbacks.log",
    fun = function(msg) invisible(NULL),
    params = list(msg = "string"),
    results = NULL
  )

component <- wt_component("stats_plugin.component.wasm") |>
  wt_with_runtime(rt) |>
  wt_with_wit("world.wit", world = "stats") |>
  wt_with_wasi(wasi) |>
  wt_with_limits(limits) |>
  wt_with_callbacks(callbacks) |>
  wt_with_arrays(default_dtype = "f64", layout = "column-major", transport = "arena") |>
  wt_prepare()

expect_class(component, "WtPreparedApp")
expect_class(component$spec, "WtComponentSpec")
component_print <- capture.output(returned <- print(component$spec))
expect_identical(returned, component$spec)
expect_true(grepl("<WtComponentSpec>", component_print[[1L]], fixed = TRUE))
expect_equal(component$spec$source, "stats_plugin.component.wasm")
expect_equal(component$spec$wit$wit, "world.wit")
expect_equal(component$spec$wit$world, "stats")
expect_equal(length(component$spec$wasi$args), 1L)
expect_equal(component$spec$limits$memory_bytes, 128 * 1024^2)
expect_equal(length(component$spec$callbacks$entries), 1L)
expect_equal(component$spec$arrays$transport, "arena")

for (expr in list(
  quote(component |> wt_component_exports()),
  quote(component |> wt_component_imports())
)) {
  err <- tryCatch(eval(expr), error = identity)
  if (identical(rt$backend, "native")) {
    expect_true(inherits(err, "rwasmtime_compile_error"))
    expect_true(grepl("failed to compile Wasm component", conditionMessage(err), fixed = TRUE))
  } else {
    expect_true(inherits(err, "rwasmtime_not_implemented"))
  }
}

component_rt <- wt_runtime_spec() |>
  wt_with_compiler("cranelift", opt_level = "speed") |>
  wt_enable_features(component_model = TRUE, simd = TRUE, relaxed_simd = FALSE) |>
  wt_build_runtime()
if (identical(component_rt$backend, "native")) {
  import_component_wat <- '
  (component
    (import "host-add" (func (param "x" s32) (param "y" s32) (result s32))))
  '
  export_component_wat <- '
  (component
    (core module $m
      (func (export "answer") (result i32)
        i32.const 42))
    (core instance $i (instantiate $m))
    (func $answer (result s32) (canon lift (core func $i "answer")))
    (export "answer" (func $answer)))
  '
  imports <- wt_component(import_component_wat) |>
    wt_with_runtime(component_rt) |>
    wt_component_imports()
  expect_equal(length(imports), 1L)
  expect_class(imports[[1L]], "WtComponentItem")
  expect_equal(imports[[1L]]$name, "host-add")
  expect_equal(imports[[1L]]$kind, "function")
  expect_equal(imports[[1L]]$params_schema, "x: s32, y: s32")
  expect_equal(imports[[1L]]$results_schema, "s32")
  expect_true(grepl("<WtComponentItem>", capture.output(print(imports[[1L]]))[[1L]], fixed = TRUE))

  prepared_exports <- wt_component(export_component_wat) |>
    wt_with_runtime(component_rt) |>
    wt_prepare() |>
    wt_component_exports()
  expect_equal(length(prepared_exports), 1L)
  expect_equal(prepared_exports[[1L]]$name, "answer")
  expect_equal(prepared_exports[[1L]]$kind, "function")
  expect_equal(prepared_exports[[1L]]$results_schema, "s32")
}
