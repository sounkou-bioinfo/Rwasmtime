source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

rt <- wt_runtime_spec() |>
  wt_with_compiler("cranelift", opt_level = "speed") |>
  wt_build_runtime()

wasi <- wt_wasi() |>
  wt_wasi_stdio(stdout = "capture", stderr = "capture")

callbacks <- wt_callbacks() |>
  wt_add_callback(
    module = "r",
    name = "score_f64",
    fun = function(x, y) x + y,
    params = c("f64", "f64"),
    results = "f64",
    abi = "core"
  )

artifact <- rt |>
  wt_compile("add.wasm", kind = "module")
store <- rt |>
  wt_store(limits = wt_limits() |> wt_limit_memory("64MiB"), wasi = wasi)
linker <- rt |>
  wt_linker() |>
  wt_link_wasi(wasi) |>
  wt_link_callbacks(callbacks)
instance <- artifact |>
  wt_instantiate(store = store, linker = linker)

expect_class(artifact, "WtArtifact")
artifact_app <- wt_app(artifact)
artifact_app_print <- capture.output(returned <- print(artifact_app))
expect_identical(returned, artifact_app)
expect_true(grepl("source=<WtArtifact:module>", artifact_app_print[[1L]], fixed = TRUE))
info <- wt_artifact_info(artifact)
expect_class(info, "WtArtifactInfo")
printed <- capture.output(returned <- print(info))
expect_identical(returned, info)
expect_true(grepl("<WtArtifactInfo>", printed[[1L]], fixed = TRUE))
expect_equal(info$metadata$format_version, 1L)
expect_equal(info$metadata$compiler$strategy, "cranelift")
expect_equal(info$metadata$compiler$opt_level, "speed")
expect_true(wt_artifact_compatible(artifact, rt))
rt_incompatible <- wt_runtime_spec() |>
  wt_with_compiler("winch", opt_level = "none") |>
  wt_enable_features(simd = FALSE) |>
  wt_build_runtime()
expect_false(wt_artifact_compatible(artifact, rt_incompatible))

expect_class(store, "WtStore")
expect_true(grepl("<WtStore>", capture.output(print(store))[[1L]], fixed = TRUE))
expect_class(linker, "WtLinker")
expect_true(grepl("<WtLinker>", capture.output(print(linker))[[1L]], fixed = TRUE))
expect_class(instance, "WtInstance")

mem <- instance |>
  wt_memory("memory")
expect_class(mem, "WtMemory")
expect_true(grepl("<WtMemory>", capture.output(print(mem))[[1L]], fixed = TRUE))

err <- tryCatch(instance |> wt_call("add", 1L, 2L), error = identity)
expect_true(inherits(err, "rwasmtime_not_implemented"))

job <- instance |>
  wt_call_async("add", 1L, 2L)
expect_class(job, "WtJob")
status <- wt_poll(job)
expect_class(status, "WtJobStatus")
expect_true(grepl("<WtJobStatus>", capture.output(print(status))[[1L]], fixed = TRUE))
expect_equal(status$state, "pending")
