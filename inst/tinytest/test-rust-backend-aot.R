source(system.file("tinytest", "helper-rwasmtime.R", package = "Rwasmtime"))

add_wat <- '
(module
  (func (export "add") (param i32 i32) (result i32)
    local.get 0
    local.get 1
    i32.add))
'

rt <- rwasmtime_backend_runtime()
if (!identical(rt$backend, "native")) exit_file("native Rust/Wasmtime backend is not available in this install")
path <- tempfile(fileext = ".cwasm")
artifact <- rt |>
  wt_compile(add_wat, kind = "module") |>
  wt_aot_save(path, overwrite = TRUE)

expect_true(file.exists(path))
meta_path <- paste0(path, ".rwasmtime.rds")
expect_true(file.exists(meta_path))
expect_true(file.info(path)$size > 0)
expect_equal(artifact$aot_path, path)

loaded <- rt |> wt_aot_load(path)
expect_class(loaded, "WtArtifact")
expect_equal(loaded$backend, "native")
expect_true(wt_artifact_compatible(loaded, rt))

store <- rt |> wt_store()
linker <- rt |> wt_linker()
instance <- loaded |> wt_instantiate(store = store, linker = linker)
expect_equal(wt_call(instance, "add", 20L, 22L), 42L)

prepared <- wt_app(loaded) |> wt_prepare()
expect_equal(prepared$backend, "native")
expect_equal(wt_call(prepared, "add", 20L, 22L), 42L)
session <- prepared |> wt_new_session()
expect_equal(wt_call(session, "add", 20L, 22L), 42L)
job <- prepared |> wt_call_async("add", 20L, 22L)
expect_true(isTRUE(wt_poll(job)$done))
expect_equal(job |> wt_await(), 42L)

bad_meta <- readRDS(meta_path)
bad_meta$features$simd <- !isTRUE(bad_meta$features$simd)
saveRDS(bad_meta, meta_path)
err <- expect_error_class(rt |> wt_aot_load(path), "rwasmtime_aot_incompatible")
expect_true(grepl("incompatible", conditionMessage(err), fixed = TRUE))
