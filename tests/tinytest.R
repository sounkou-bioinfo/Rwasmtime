if (requireNamespace("tinytest", quietly = TRUE)) {
  pattern <- Sys.getenv("RWASMTIME_TINYTEST_PATTERN", unset = "")
  require_results <- identical(Sys.getenv("RWASMTIME_TINYTEST_REQUIRE_RESULTS"), "true")

  results <- if (nzchar(pattern)) {
    tinytest::test_package("Rwasmtime", pattern = pattern, ncpu = 1L)
  } else {
    tinytest::test_package("Rwasmtime", ncpu = 1L)
  }

  if (isTRUE(require_results) && !length(results)) {
    stop("tinytest produced zero results for pattern: ", pattern, call. = FALSE)
  }
}
