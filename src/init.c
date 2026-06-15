
// clang-format sorts includes unless SortIncludes: Never. However, the ordering
// does matter here. So, we need to disable clang-format for safety.

// clang-format off
#include <stdint.h>
#include <Rinternals.h>
#include <R_ext/Parse.h>
// clang-format on

#include "rust/api.h"

static uintptr_t TAGGED_POINTER_MASK = (uintptr_t)1;

SEXP handle_result(SEXP res_) {
    uintptr_t res = (uintptr_t)res_;

    // An error is indicated by tag.
    if ((res & TAGGED_POINTER_MASK) == 1) {
        // Remove tag
        SEXP res_aligned = (SEXP)(res & ~TAGGED_POINTER_MASK);

        // Currently, there are two types of error cases:
        //
        //   1. Error from Rust code
        //   2. Error from R's C API, which is caught by R_UnwindProtect()
        //
        if (TYPEOF(res_aligned) == CHARSXP) {
            // In case 1, the result is an error message that can be passed to
            // Rf_errorcall() directly.
            Rf_errorcall(R_NilValue, "%s", CHAR(res_aligned));
        } else {
            // In case 2, the result is the token to restart the
            // cleanup process on R's side.
            R_ContinueUnwind(res_aligned);
        }
    }

    return (SEXP)res;
}

SEXP savvy_rwasmtime_backend_status__impl(void) {
    SEXP res = savvy_rwasmtime_backend_status__ffi();
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeInstance_call_core__impl(SEXP self__, SEXP c_arg__export, SEXP c_arg__args) {
    SEXP res = savvy_RwasmtimeNativeInstance_call_core__ffi(self__, c_arg__export, c_arg__args);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeInstance_memory_grow__impl(SEXP self__, SEXP c_arg__name, SEXP c_arg__pages) {
    SEXP res = savvy_RwasmtimeNativeInstance_memory_grow__ffi(self__, c_arg__name, c_arg__pages);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeInstance_memory_read__impl(SEXP self__, SEXP c_arg__name, SEXP c_arg__offset, SEXP c_arg__len) {
    SEXP res = savvy_RwasmtimeNativeInstance_memory_read__ffi(self__, c_arg__name, c_arg__offset, c_arg__len);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeInstance_memory_size__impl(SEXP self__, SEXP c_arg__name) {
    SEXP res = savvy_RwasmtimeNativeInstance_memory_size__ffi(self__, c_arg__name);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeInstance_memory_write__impl(SEXP self__, SEXP c_arg__name, SEXP c_arg__offset, SEXP c_arg__value) {
    SEXP res = savvy_RwasmtimeNativeInstance_memory_write__ffi(self__, c_arg__name, c_arg__offset, c_arg__value);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeInstance_wasi_output__impl(SEXP self__) {
    SEXP res = savvy_RwasmtimeNativeInstance_wasi_output__ffi(self__);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeModule_instantiate__impl(SEXP self__, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms) {
    SEXP res = savvy_RwasmtimeNativeModule_instantiate__ffi(self__, c_arg__memory_bytes, c_arg__table_elements, c_arg__instances, c_arg__fuel, c_arg__wall_time_ms);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeModule_instantiate_callbacks__impl(SEXP self__, SEXP c_arg__callback_modules, SEXP c_arg__callback_names, SEXP c_arg__callback_functions, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms) {
    SEXP res = savvy_RwasmtimeNativeModule_instantiate_callbacks__ffi(self__, c_arg__callback_modules, c_arg__callback_names, c_arg__callback_functions, c_arg__memory_bytes, c_arg__table_elements, c_arg__instances, c_arg__fuel, c_arg__wall_time_ms);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeModule_instantiate_wasi_p1__impl(SEXP self__, SEXP c_arg__args, SEXP c_arg__env_names, SEXP c_arg__env_values, SEXP c_arg__preopen_guest, SEXP c_arg__preopen_host, SEXP c_arg__preopen_readonly, SEXP c_arg__stdin, SEXP c_arg__stdout, SEXP c_arg__stderr, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms, SEXP c_arg__input) {
    SEXP res = savvy_RwasmtimeNativeModule_instantiate_wasi_p1__ffi(self__, c_arg__args, c_arg__env_names, c_arg__env_values, c_arg__preopen_guest, c_arg__preopen_host, c_arg__preopen_readonly, c_arg__stdin, c_arg__stdout, c_arg__stderr, c_arg__memory_bytes, c_arg__table_elements, c_arg__instances, c_arg__fuel, c_arg__wall_time_ms, c_arg__input);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeModule_serialize__impl(SEXP self__) {
    SEXP res = savvy_RwasmtimeNativeModule_serialize__ffi(self__);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeRuntime_build__impl(SEXP c_arg__compiler_strategy, SEXP c_arg__opt_level, SEXP c_arg__parallel, SEXP c_arg__component_model, SEXP c_arg__component_model_async, SEXP c_arg__simd, SEXP c_arg__relaxed_simd, SEXP c_arg__relaxed_simd_deterministic, SEXP c_arg__bulk_memory, SEXP c_arg__multi_memory, SEXP c_arg__memory64, SEXP c_arg__threads, SEXP c_arg__gc) {
    SEXP res = savvy_RwasmtimeNativeRuntime_build__ffi(c_arg__compiler_strategy, c_arg__opt_level, c_arg__parallel, c_arg__component_model, c_arg__component_model_async, c_arg__simd, c_arg__relaxed_simd, c_arg__relaxed_simd_deterministic, c_arg__bulk_memory, c_arg__multi_memory, c_arg__memory64, c_arg__threads, c_arg__gc);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeRuntime_call_core__impl(SEXP self__, SEXP c_arg__module, SEXP c_arg__export, SEXP c_arg__args, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms) {
    SEXP res = savvy_RwasmtimeNativeRuntime_call_core__ffi(self__, c_arg__module, c_arg__export, c_arg__args, c_arg__memory_bytes, c_arg__table_elements, c_arg__instances, c_arg__fuel, c_arg__wall_time_ms);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeRuntime_compile_core__impl(SEXP self__, SEXP c_arg__module) {
    SEXP res = savvy_RwasmtimeNativeRuntime_compile_core__ffi(self__, c_arg__module);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeRuntime_component_exports__impl(SEXP self__, SEXP c_arg__component) {
    SEXP res = savvy_RwasmtimeNativeRuntime_component_exports__ffi(self__, c_arg__component);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeRuntime_component_imports__impl(SEXP self__, SEXP c_arg__component) {
    SEXP res = savvy_RwasmtimeNativeRuntime_component_imports__ffi(self__, c_arg__component);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeRuntime_deserialize_core__impl(SEXP self__, SEXP c_arg__bytes) {
    SEXP res = savvy_RwasmtimeNativeRuntime_deserialize_core__ffi(self__, c_arg__bytes);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeRuntime_instantiate_core__impl(SEXP self__, SEXP c_arg__module, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms) {
    SEXP res = savvy_RwasmtimeNativeRuntime_instantiate_core__ffi(self__, c_arg__module, c_arg__memory_bytes, c_arg__table_elements, c_arg__instances, c_arg__fuel, c_arg__wall_time_ms);
    return handle_result(res);
}

SEXP savvy_RwasmtimeNativeRuntime_run_wasi_p1__impl(SEXP self__, SEXP c_arg__module, SEXP c_arg__args, SEXP c_arg__env_names, SEXP c_arg__env_values, SEXP c_arg__preopen_guest, SEXP c_arg__preopen_host, SEXP c_arg__preopen_readonly, SEXP c_arg__stdin, SEXP c_arg__stdout, SEXP c_arg__stderr, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms, SEXP c_arg__input) {
    SEXP res = savvy_RwasmtimeNativeRuntime_run_wasi_p1__ffi(self__, c_arg__module, c_arg__args, c_arg__env_names, c_arg__env_values, c_arg__preopen_guest, c_arg__preopen_host, c_arg__preopen_readonly, c_arg__stdin, c_arg__stdout, c_arg__stderr, c_arg__memory_bytes, c_arg__table_elements, c_arg__instances, c_arg__fuel, c_arg__wall_time_ms, c_arg__input);
    return handle_result(res);
}


static const R_CallMethodDef CallEntries[] = {
    {"savvy_rwasmtime_backend_status__impl", (DL_FUNC) &savvy_rwasmtime_backend_status__impl, 0},
    {"savvy_RwasmtimeNativeInstance_call_core__impl", (DL_FUNC) &savvy_RwasmtimeNativeInstance_call_core__impl, 3},
    {"savvy_RwasmtimeNativeInstance_memory_grow__impl", (DL_FUNC) &savvy_RwasmtimeNativeInstance_memory_grow__impl, 3},
    {"savvy_RwasmtimeNativeInstance_memory_read__impl", (DL_FUNC) &savvy_RwasmtimeNativeInstance_memory_read__impl, 4},
    {"savvy_RwasmtimeNativeInstance_memory_size__impl", (DL_FUNC) &savvy_RwasmtimeNativeInstance_memory_size__impl, 2},
    {"savvy_RwasmtimeNativeInstance_memory_write__impl", (DL_FUNC) &savvy_RwasmtimeNativeInstance_memory_write__impl, 4},
    {"savvy_RwasmtimeNativeInstance_wasi_output__impl", (DL_FUNC) &savvy_RwasmtimeNativeInstance_wasi_output__impl, 1},
    {"savvy_RwasmtimeNativeModule_instantiate__impl", (DL_FUNC) &savvy_RwasmtimeNativeModule_instantiate__impl, 6},
    {"savvy_RwasmtimeNativeModule_instantiate_callbacks__impl", (DL_FUNC) &savvy_RwasmtimeNativeModule_instantiate_callbacks__impl, 9},
    {"savvy_RwasmtimeNativeModule_instantiate_wasi_p1__impl", (DL_FUNC) &savvy_RwasmtimeNativeModule_instantiate_wasi_p1__impl, 16},
    {"savvy_RwasmtimeNativeModule_serialize__impl", (DL_FUNC) &savvy_RwasmtimeNativeModule_serialize__impl, 1},
    {"savvy_RwasmtimeNativeRuntime_build__impl", (DL_FUNC) &savvy_RwasmtimeNativeRuntime_build__impl, 13},
    {"savvy_RwasmtimeNativeRuntime_call_core__impl", (DL_FUNC) &savvy_RwasmtimeNativeRuntime_call_core__impl, 9},
    {"savvy_RwasmtimeNativeRuntime_compile_core__impl", (DL_FUNC) &savvy_RwasmtimeNativeRuntime_compile_core__impl, 2},
    {"savvy_RwasmtimeNativeRuntime_component_exports__impl", (DL_FUNC) &savvy_RwasmtimeNativeRuntime_component_exports__impl, 2},
    {"savvy_RwasmtimeNativeRuntime_component_imports__impl", (DL_FUNC) &savvy_RwasmtimeNativeRuntime_component_imports__impl, 2},
    {"savvy_RwasmtimeNativeRuntime_deserialize_core__impl", (DL_FUNC) &savvy_RwasmtimeNativeRuntime_deserialize_core__impl, 2},
    {"savvy_RwasmtimeNativeRuntime_instantiate_core__impl", (DL_FUNC) &savvy_RwasmtimeNativeRuntime_instantiate_core__impl, 7},
    {"savvy_RwasmtimeNativeRuntime_run_wasi_p1__impl", (DL_FUNC) &savvy_RwasmtimeNativeRuntime_run_wasi_p1__impl, 17},
    {NULL, NULL, 0}
};

void R_init_Rwasmtime(DllInfo *dll) {
    R_registerRoutines(dll, NULL, CallEntries, NULL, NULL);
    R_useDynamicSymbols(dll, FALSE);

    // Functions for initialization, if any.

}
