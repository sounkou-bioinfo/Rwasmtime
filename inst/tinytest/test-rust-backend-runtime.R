source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

rt <- rwasmtime_backend_runtime()
if (!identical(rt$backend, "native")) exit_file("native Rust/Wasmtime backend is not available in this install")
expect_class(rt, "WtRuntime")
expect_equal(rt$backend, "native")
expect_false(is.null(rt$ptr))
expect_class(rt$ptr, "RwasmtimeNativeRuntime")

bad <- wt_runtime_spec()
bad$features$relaxed_simd <- FALSE
bad$features$relaxed_simd_deterministic <- TRUE
err <- expect_error_class(wt_build_runtime(bad), "error")
expect_true(grepl("relaxed_simd_deterministic", conditionMessage(err), fixed = TRUE))
