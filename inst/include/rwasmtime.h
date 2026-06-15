#ifndef RWASMTIME_H
#define RWASMTIME_H

/*
 * Public C API boundary for Rwasmtime.
 *
 * This header is installed as inst/include/rwasmtime.h for downstream native
 * packages. It is intentionally pure C: it does not include R headers, does not
 * expose SEXP, and does not permit worker-thread callbacks to call R's C API.
 *
 * Current reality: the default package build keeps execution calls as honest
 * not-implemented boundaries. Native-backend source builds wire
 * rwasmtime_runtime_build() to the experimental Rust/Wasmtime engine builder
 * and rwasmtime_runtime_call_core() to a deliberately tiny one-shot core Wasm
 * call boundary. Do not expand this header speculatively; grow it only when
 * the Rust/Wasmtime boundary it represents is implemented or under active
 * implementation.
 *
 * Ownership rule: any function that writes a non-NULL handle through an out
 * parameter transfers one ownership reference to the caller. Release it with
 * the matching rwasmtime_*_release() function. Passing NULL to release
 * functions is valid and has no effect.
 *
 * Error rule: when a function accepts rwasmtime_error_t **err and returns a
 * non-OK status, it may store a newly allocated error in *err. The caller owns
 * that error and must release it with rwasmtime_error_release(). Borrowed error
 * strings become invalid after release.
 *
 * rwasmtime_runtime_call_core() is intentionally narrow: it compiles the copied
 * core module byte slice, instantiates it with no imports, calls one exported
 * core function, and copies scalar i32/i64/f32/f64 values in and out. It does
 * not expose WASI, callbacks, components, WIT values, persistent stores,
 * memory, tables, or host references. It is not a sandbox boundary unless the
 * caller supplies explicit limits in rwasmtime_core_call_options_t.
 */

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifndef RWASMTIME_API
#if defined(_WIN32) && defined(RWASMTIME_BUILDING)
#define RWASMTIME_API __declspec(dllexport)
#else
#define RWASMTIME_API
#endif
#endif

#define RWASMTIME_C_API_VERSION 2u

#define RWASMTIME_CORE_VALUE_I32 1u
#define RWASMTIME_CORE_VALUE_I64 2u
#define RWASMTIME_CORE_VALUE_F32 3u
#define RWASMTIME_CORE_VALUE_F64 4u

typedef struct rwasmtime_runtime rwasmtime_runtime_t;
typedef struct rwasmtime_error rwasmtime_error_t;

typedef enum rwasmtime_status {
  RWASMTIME_OK = 0,
  RWASMTIME_ERR = 1,
  RWASMTIME_INVALID_ARGUMENT = 2,
  RWASMTIME_NOT_IMPLEMENTED = 3
} rwasmtime_status_t;

typedef enum rwasmtime_toggle {
  RWASMTIME_TOGGLE_UNSET = 0,
  RWASMTIME_TOGGLE_FALSE = 1,
  RWASMTIME_TOGGLE_TRUE = 2
} rwasmtime_toggle_t;

typedef struct rwasmtime_runtime_options {
  size_t struct_size;
  const char *compiler_strategy; /* "auto", "cranelift", or "winch". */
  const char *opt_level;         /* "none", "speed", or "speed_and_size". */
  rwasmtime_toggle_t parallel;
  rwasmtime_toggle_t component_model;
  rwasmtime_toggle_t simd;
  rwasmtime_toggle_t relaxed_simd;
  rwasmtime_toggle_t relaxed_simd_deterministic;
} rwasmtime_runtime_options_t;

typedef struct rwasmtime_core_value {
  uint32_t tag;       /* RWASMTIME_CORE_VALUE_* */
  int64_t i64_value;  /* i32/i64 payload and exact integer transport. */
  double f64_value;   /* f32/f64 payload and optional numeric mirror. */
} rwasmtime_core_value_t;

typedef struct rwasmtime_core_call_options {
  size_t struct_size;
  /* Explicit zero is meaningful when the matching has_* flag is non-zero. */
  int has_memory_bytes;
  uint64_t memory_bytes;
  int has_table_elements;
  uint64_t table_elements;
  int has_instances;
  uint64_t instances;
  int has_fuel;
  uint64_t fuel;
  int has_wall_time_ms;
  uint64_t wall_time_ms;
} rwasmtime_core_call_options_t;

RWASMTIME_API uint32_t rwasmtime_c_api_version(void);
RWASMTIME_API const char *rwasmtime_status_name(rwasmtime_status_t status);
RWASMTIME_API rwasmtime_status_t rwasmtime_error_status(const rwasmtime_error_t *err);
RWASMTIME_API const char *rwasmtime_error_message(const rwasmtime_error_t *err);
RWASMTIME_API void rwasmtime_error_release(rwasmtime_error_t *err);

RWASMTIME_API rwasmtime_status_t rwasmtime_runtime_build(
    const rwasmtime_runtime_options_t *opts,
    rwasmtime_runtime_t **out,
    rwasmtime_error_t **err);
RWASMTIME_API rwasmtime_status_t rwasmtime_runtime_call_core(
    rwasmtime_runtime_t *runtime,
    const uint8_t *module_bytes,
    size_t module_len,
    const char *export_name,
    const rwasmtime_core_value_t *args,
    size_t args_len,
    const rwasmtime_core_call_options_t *opts,
    rwasmtime_core_value_t *results,
    size_t results_capacity,
    size_t *results_len,
    rwasmtime_error_t **err);
RWASMTIME_API void rwasmtime_runtime_release(rwasmtime_runtime_t *runtime);

#ifdef __cplusplus
}
#endif

#endif /* RWASMTIME_H */
