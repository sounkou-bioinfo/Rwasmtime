#define RWASMTIME_BUILDING 1
#include "../inst/include/rwasmtime.h"

#include <stdlib.h>
#include <string.h>

#ifdef RWASMTIME_WITH_RUST_BACKEND
typedef struct rwasmtime_backend_runtime rwasmtime_backend_runtime_t;
extern int rwasmtime_backend_runtime_build(
    const rwasmtime_runtime_options_t *opts,
    rwasmtime_backend_runtime_t **out,
    char **message);
extern int rwasmtime_backend_call_core(
    rwasmtime_backend_runtime_t *runtime,
    const uint8_t *module_bytes,
    size_t module_len,
    const char *export_name,
    const rwasmtime_core_value_t *args,
    size_t args_len,
    const rwasmtime_core_call_options_t *opts,
    rwasmtime_core_value_t *results,
    size_t results_capacity,
    size_t *results_len,
    char **message);
extern void rwasmtime_backend_runtime_release(rwasmtime_backend_runtime_t *runtime);
extern void rwasmtime_backend_string_release(char *value);
#endif

struct rwasmtime_runtime {
#ifdef RWASMTIME_WITH_RUST_BACKEND
  rwasmtime_backend_runtime_t *backend;
#else
  int reserved;
#endif
};

struct rwasmtime_error {
  rwasmtime_status_t status;
  char *message;
};

static char *rwasmtime_strdup(const char *src) {
  size_t n;
  char *dst;

  if (src == NULL) src = "";
  n = strlen(src) + 1u;
  dst = (char *)malloc(n);
  if (dst == NULL) return NULL;
  memcpy(dst, src, n);
  return dst;
}

static rwasmtime_status_t rwasmtime_set_error(
    rwasmtime_status_t status,
    const char *message,
    rwasmtime_error_t **err) {
  rwasmtime_error_t *value;

  if (err == NULL) return status;
  *err = NULL;
  value = (rwasmtime_error_t *)calloc(1u, sizeof(*value));
  if (value == NULL) return status;
  value->status = status;
  value->message = rwasmtime_strdup(message);
  *err = value;
  return status;
}

uint32_t rwasmtime_c_api_version(void) {
  return RWASMTIME_C_API_VERSION;
}

const char *rwasmtime_status_name(rwasmtime_status_t status) {
  switch (status) {
  case RWASMTIME_OK:
    return "ok";
  case RWASMTIME_ERR:
    return "error";
  case RWASMTIME_INVALID_ARGUMENT:
    return "invalid_argument";
  case RWASMTIME_NOT_IMPLEMENTED:
    return "not_implemented";
  default:
    return "unknown";
  }
}

rwasmtime_status_t rwasmtime_error_status(const rwasmtime_error_t *err) {
  if (err == NULL) return RWASMTIME_OK;
  return err->status;
}

const char *rwasmtime_error_message(const rwasmtime_error_t *err) {
  if (err == NULL || err->message == NULL) return "";
  return err->message;
}

void rwasmtime_error_release(rwasmtime_error_t *err) {
  if (err == NULL) return;
  free(err->message);
  free(err);
}

rwasmtime_status_t rwasmtime_runtime_build(
    const rwasmtime_runtime_options_t *opts,
    rwasmtime_runtime_t **out,
    rwasmtime_error_t **err) {
  if (out == NULL) {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "out must not be NULL",
        err);
  }

  *out = NULL;
  if (opts != NULL && opts->struct_size != sizeof(*opts)) {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "rwasmtime_runtime_options_t has an unsupported struct_size",
        err);
  }

#ifdef RWASMTIME_WITH_RUST_BACKEND
  {
    rwasmtime_backend_runtime_t *backend = NULL;
    rwasmtime_runtime_t *runtime = NULL;
    char *message = NULL;
    rwasmtime_status_t status = (rwasmtime_status_t)rwasmtime_backend_runtime_build(
        opts,
        &backend,
        &message);
    if (status != RWASMTIME_OK) {
      rwasmtime_status_t out_status = rwasmtime_set_error(
          status,
          message != NULL ? message : "rwasmtime_runtime_build failed",
          err);
      rwasmtime_backend_string_release(message);
      return out_status;
    }

    runtime = (rwasmtime_runtime_t *)calloc(1u, sizeof(*runtime));
    if (runtime == NULL) {
      rwasmtime_backend_runtime_release(backend);
      return rwasmtime_set_error(
          RWASMTIME_ERR,
          "failed to allocate rwasmtime runtime wrapper",
          err);
    }
    runtime->backend = backend;
    *out = runtime;
    return RWASMTIME_OK;
  }
#else
  return rwasmtime_set_error(
      RWASMTIME_NOT_IMPLEMENTED,
      "rwasmtime_runtime_build is not implemented in this C-only package build; install a native Rust/Wasmtime backend build to use this boundary",
      err);
#endif
}

rwasmtime_status_t rwasmtime_runtime_call_core(
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
    rwasmtime_error_t **err) {
  if (err != NULL) *err = NULL;
  if (results_len != NULL) *results_len = 0u;

  if (runtime == NULL) {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "runtime must not be NULL",
        err);
  }
  if (module_len > 0u && module_bytes == NULL) {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "module_bytes must not be NULL when module_len is non-zero",
        err);
  }
  if (module_len == 0u) {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "module_len must be non-zero",
        err);
  }
  if (export_name == NULL || export_name[0] == '\0') {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "export_name must not be NULL or empty",
        err);
  }
  if (args_len > 0u && args == NULL) {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "args must not be NULL when args_len is non-zero",
        err);
  }
  if (results_capacity > 0u && results == NULL) {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "results must not be NULL when results_capacity is non-zero",
        err);
  }
  if (results_len == NULL) {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "results_len must not be NULL",
        err);
  }
  if (opts != NULL && opts->struct_size != sizeof(*opts)) {
    return rwasmtime_set_error(
        RWASMTIME_INVALID_ARGUMENT,
        "rwasmtime_core_call_options_t has an unsupported struct_size",
        err);
  }

#ifdef RWASMTIME_WITH_RUST_BACKEND
  {
    char *message = NULL;
    rwasmtime_status_t status;
    if (runtime->backend == NULL) {
      return rwasmtime_set_error(
          RWASMTIME_INVALID_ARGUMENT,
          "runtime backend must not be NULL",
          err);
    }
    status = (rwasmtime_status_t)rwasmtime_backend_call_core(
        runtime->backend,
        module_bytes,
        module_len,
        export_name,
        args,
        args_len,
        opts,
        results,
        results_capacity,
        results_len,
        &message);
    if (status != RWASMTIME_OK) {
      rwasmtime_status_t out_status = rwasmtime_set_error(
          status,
          message != NULL ? message : "rwasmtime_runtime_call_core failed",
          err);
      rwasmtime_backend_string_release(message);
      return out_status;
    }
    return RWASMTIME_OK;
  }
#else
  return rwasmtime_set_error(
      RWASMTIME_NOT_IMPLEMENTED,
      "rwasmtime_runtime_call_core is not implemented in this C-only package build; install a native Rust/Wasmtime backend build to use this boundary",
      err);
#endif
}

void rwasmtime_runtime_release(rwasmtime_runtime_t *runtime) {
  if (runtime == NULL) return;
#ifdef RWASMTIME_WITH_RUST_BACKEND
  rwasmtime_backend_runtime_release(runtime->backend);
#endif
  free(runtime);
}
