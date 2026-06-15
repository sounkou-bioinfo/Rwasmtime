source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

app <- wt_app("stats_plugin.component.wasm") |>
  wt_as_component() |>
  wt_prepare()
session <- app |>
  wt_new_session()
mem <- session |>
  wt_memory("memory")
expect_class(mem, "WtMemory")
expect_true(grepl("<WtMemory>", capture.output(print(mem))[[1L]], fixed = TRUE))

view <- mem |>
  wt_memory_view(
    ptr = 1024,
    length = 1000,
    dtype = "f64",
    mutable = FALSE,
    lifetime = "until_next_wasm_call"
  )
expect_class(view, "WtMemoryView")
expect_true(grepl("<WtMemoryView>", capture.output(print(view))[[1L]], fixed = TRUE))
expect_equal(view$ptr, 1024)
expect_equal(view$length, 1000)
expect_equal(view$dtype, "f64")
expect_false(view$mutable)
expect_equal(view$lifetime, "until_next_wasm_call")

session <- session |>
  wt_with_temp_array(
    name = "x",
    value = c(1, 2, 3),
    dtype = "f64",
    layout = "column-major"
  )
expect_equal(names(session$temp_arrays), "x")
expect_equal(session$temp_arrays$x$dtype, "f64")
expect_equal(session$temp_arrays$x$layout, "column-major")

arg <- wt_arg_array("x")
expect_class(arg, "WtArrayArgument")
expect_equal(arg$name, "x")
arg_print <- capture.output(returned <- print(arg))
expect_identical(returned, arg)
expect_true(grepl("<WtArrayArgument>", arg_print[[1L]], fixed = TRUE))

for (expr in list(
  quote(mem |> wt_memory_size()),
  quote(mem |> wt_memory_grow(1L)),
  quote(mem |> wt_memory_read(ptr = 0, length = 1, dtype = "u8")),
  quote(mem |> wt_memory_write(ptr = 0, value = as.raw(1), dtype = "u8")),
  quote(session |> wt_array_write(c(1, 2), dtype = "f64")),
  quote(arg |> wt_as_array(dtype = "f64", dim = c(2)))
)) {
  err <- tryCatch(eval(expr), error = identity)
  expect_true(inherits(err, "rwasmtime_not_implemented"))
}
