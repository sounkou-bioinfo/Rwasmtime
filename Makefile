PKGNAME := $(shell sed -n 's/Package: *\([^ ]*\)/\1/p' DESCRIPTION 2>/dev/null)
PKGVERS := $(shell sed -n 's/Version: *\([^ ]*\)/\1/p' DESCRIPTION 2>/dev/null)

all: help

help:
	@printf '%s\n' \
	  'Common development targets:' \
	  '  make rd          regenerate savvy wrappers, roxygen docs, and NAMESPACE' \
	  '  make rdm         render README.Rmd to README.md with native backend examples' \
	  '  make authors     regenerate inst/AUTHORS and inst/LICENCE.note from Cargo metadata' \
	  '  make dev-install install current source with configure/preclean' \
	  '  make test-api-surface verify intended R API exports/docs' \
	  '  make test-fast   run non-network tinytest' \
	  '  make test-rust   verify R-free Rust core boundary, then run Rust unit tests' \
	  '  make test-rust-backend run feature-gated real Wasmtime/Cranelift smoke tests' \
	  '  make test-rust-wasi run feature-gated real WASIp1 smoke tests' \
	  '  make test-c-api  run C API symbol, R-free boundary, Rtinycc header, and installed-symbol checks' \
	  '  make test-c-api-rust-backend install with Rust/Wasmtime backend and exercise real runtime_build/call_core' \
	  '  make test-r-runtime-rust-backend verify wt_build_runtime returns a native Savvy runtime handle' \
	  '  make test-r-aot-rust-backend verify core AOT save/load roundtrip and metadata gate' \
	  '  make test-r-call-rust-backend verify wt_call executes a real core Wasm function' \
	  '  make test-r-exec-rust-backend verify wt_exec keeps persistent side-effect calls pipeable' \
	  '  make test-r-low-level-rust-backend verify wt_compile/wt_instantiate native core artifacts' \
	  '  make test-r-memory-rust-backend verify persistent sessions, typed memory, and guest arrays' \
	  '  make test-r-repl-rust-backend verify core-memory persistent REPL protocol transport' \
	  '  make test-r-wasi-rust-backend verify prepared and low-level WASIp1 execution' \
	  '  make test-r-callbacks-rust-backend verify core Wasm imports call R callbacks' \
	  '  make test-r-native-rust-backend install once and run all native backend tinytests' \
	  '  make test-webr   verify wasm/webR configure selects generated-symbol stubs' \
	  '  make test        install then run tinytest' \
	  '  make check       build and run R CMD check --as-cran --no-manual' \
	  '  make clean       remove local build products'

rd:
	R -e 'if (requireNamespace("savvy", quietly = TRUE) && file.exists("src/savvy/Cargo.toml")) { status <- system2(getFromNamespace("savvy_cli_path", "savvy")(), c("update", ".", "src/savvy")); if (!identical(status, 0L)) stop("savvy-cli update failed") }; if (requireNamespace("roxygen2", quietly = TRUE)) roxygen2::roxygenize(load_code = "source") else stop("roxygen2 is required")'

rdm: dev-install-rust-backend
	R -e 'rmarkdown::render("README.Rmd")'

authors:
	Rscript tools/update-authors.R

build:
	R CMD build .

check: build
	R CMD check --as-cran --no-manual $(PKGNAME)_$(PKGVERS).tar.gz

install_deps:
	R -e 'if (!requireNamespace("remotes", quietly = TRUE)) install.packages("remotes"); remotes::install_deps(dependencies = TRUE)'

dev-install:
	R CMD INSTALL --preclean .

install: build
	R CMD INSTALL $(PKGNAME)_$(PKGVERS).tar.gz

test-api-surface:
	Rscript tools/check-api-surface.R

test-fast: test-api-surface dev-install
	Rscript tests/tinytest.R

test-rust-boundary:
	Rscript tools/check-rust-core-boundary.R

test-rust: test-rust-boundary
	cargo test --manifest-path=src/rust/Cargo.toml

test-rust-backend: test-rust-boundary
	cargo test --manifest-path=src/rust/Cargo.toml --features wasmtime

test-rust-wasi: test-rust-boundary
	cargo test --manifest-path=src/rust/Cargo.toml --features wasi

test-c-api-symbols:
	Rscript tools/check-c-api-symbols.R inst/include/rwasmtime.h src/c_api.c

test-c-api-boundary:
	Rscript tools/check-c-api-boundary.R inst/include/rwasmtime.h src/c_api.c

test-c-api-header:
	Rscript tools/check-c-api-header.R inst/include/rwasmtime.h

test-c-api-roundtrip: dev-install
	Rscript tools/check-c-api-roundtrip.R

dev-install-rust-backend:
	RWASMTIME_RUST_BACKEND=1 R CMD INSTALL --preclean .

test-c-api-rust-backend: test-c-api-symbols test-c-api-boundary test-c-api-header dev-install-rust-backend
	Rscript tools/check-c-api-rust-backend.R

test-r-runtime-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-runtime[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-r-aot-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-aot[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-r-call-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-call[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-r-exec-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-exec[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-r-low-level-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-low-level[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-r-memory-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-memory[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-r-repl-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-repl[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-r-wasi-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-wasi[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-r-callbacks-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-callbacks[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-r-native-rust-backend: dev-install-rust-backend
	RWASMTIME_TINYTEST_PATTERN='^test-rust-backend-.*[.]R$$' RWASMTIME_TINYTEST_REQUIRE_RESULTS=true Rscript tests/tinytest.R

test-webr:
	Rscript tools/check-webr-wasm-gate.R

test-c-api: test-c-api-symbols test-c-api-boundary test-c-api-header test-c-api-roundtrip

test: test-fast

clean:
	@rm -rf $(PKGNAME)_$(PKGVERS).tar.gz $(PKGNAME).Rcheck ..Rcheck
	@rm -rf src/rust/target src/savvy/target src/Makevars src/Makevars.win
	@rm -f src/*.o src/*.so src/*.dll src/*.dylib src/symbols.rds

.PHONY: all help rd rdm authors build check install_deps dev-install install dev-install-rust-backend test-api-surface test-fast test-rust-boundary test-rust test-rust-backend test-rust-wasi test-c-api-symbols test-c-api-boundary test-c-api-header test-c-api-roundtrip test-c-api-rust-backend test-r-runtime-rust-backend test-r-aot-rust-backend test-r-call-rust-backend test-r-exec-rust-backend test-r-low-level-rust-backend test-r-memory-rust-backend test-r-repl-rust-backend test-r-wasi-rust-backend test-r-callbacks-rust-backend test-r-native-rust-backend test-webr test-c-api test clean
