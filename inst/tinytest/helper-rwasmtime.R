if (!"package:Rwasmtime" %in% search()) {
  library(Rwasmtime)
}

expect_class <- function(x, class) {
  tinytest::expect_true(inherits(x, class), info = paste("inherits", class))
}

expect_error_class <- function(expr, class = "error") {
  err <- tryCatch(eval.parent(substitute(expr)), error = identity)
  tinytest::expect_true(inherits(err, class), info = paste("inherits", class))
  err
}

rwasmtime_backend_runtime <- function() {
  wt_runtime_spec() |>
    wt_with_compiler("cranelift", opt_level = "speed") |>
    wt_enable_features(component_model = FALSE, simd = TRUE, relaxed_simd = FALSE) |>
    wt_build_runtime()
}
