SEXP savvy_rwasmtime_backend_status__ffi(void);

// methods and associated functions for RwasmtimeNativeInstance
SEXP savvy_RwasmtimeNativeInstance_call_core__ffi(SEXP self__, SEXP c_arg__export, SEXP c_arg__args);
SEXP savvy_RwasmtimeNativeInstance_memory_grow__ffi(SEXP self__, SEXP c_arg__name, SEXP c_arg__pages);
SEXP savvy_RwasmtimeNativeInstance_memory_read__ffi(SEXP self__, SEXP c_arg__name, SEXP c_arg__offset, SEXP c_arg__len);
SEXP savvy_RwasmtimeNativeInstance_memory_size__ffi(SEXP self__, SEXP c_arg__name);
SEXP savvy_RwasmtimeNativeInstance_memory_write__ffi(SEXP self__, SEXP c_arg__name, SEXP c_arg__offset, SEXP c_arg__value);
SEXP savvy_RwasmtimeNativeInstance_wasi_output__ffi(SEXP self__);

// methods and associated functions for RwasmtimeNativeModule
SEXP savvy_RwasmtimeNativeModule_exports__ffi(SEXP self__);
SEXP savvy_RwasmtimeNativeModule_imports__ffi(SEXP self__);
SEXP savvy_RwasmtimeNativeModule_instantiate__ffi(SEXP self__, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms);
SEXP savvy_RwasmtimeNativeModule_instantiate_callbacks__ffi(SEXP self__, SEXP c_arg__callback_modules, SEXP c_arg__callback_names, SEXP c_arg__callback_abis, SEXP c_arg__callback_functions, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms);
SEXP savvy_RwasmtimeNativeModule_instantiate_wasi_p1__ffi(SEXP self__, SEXP c_arg__args, SEXP c_arg__env_names, SEXP c_arg__env_values, SEXP c_arg__preopen_guest, SEXP c_arg__preopen_host, SEXP c_arg__preopen_readonly, SEXP c_arg__stdin, SEXP c_arg__stdout, SEXP c_arg__stderr, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms, SEXP c_arg__input);
SEXP savvy_RwasmtimeNativeModule_instantiate_wasi_p1_callbacks__ffi(SEXP self__, SEXP c_arg__callback_modules, SEXP c_arg__callback_names, SEXP c_arg__callback_abis, SEXP c_arg__callback_functions, SEXP c_arg__args, SEXP c_arg__env_names, SEXP c_arg__env_values, SEXP c_arg__preopen_guest, SEXP c_arg__preopen_host, SEXP c_arg__preopen_readonly, SEXP c_arg__stdin, SEXP c_arg__stdout, SEXP c_arg__stderr, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms, SEXP c_arg__input);
SEXP savvy_RwasmtimeNativeModule_serialize__ffi(SEXP self__);

// methods and associated functions for RwasmtimeNativeRuntime
SEXP savvy_RwasmtimeNativeRuntime_build__ffi(SEXP c_arg__compiler_strategy, SEXP c_arg__opt_level, SEXP c_arg__parallel, SEXP c_arg__component_model, SEXP c_arg__component_model_async, SEXP c_arg__simd, SEXP c_arg__relaxed_simd, SEXP c_arg__relaxed_simd_deterministic, SEXP c_arg__bulk_memory, SEXP c_arg__multi_memory, SEXP c_arg__memory64, SEXP c_arg__threads, SEXP c_arg__exceptions, SEXP c_arg__legacy_exceptions, SEXP c_arg__gc);
SEXP savvy_RwasmtimeNativeRuntime_call_core__ffi(SEXP self__, SEXP c_arg__module, SEXP c_arg__export, SEXP c_arg__args, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms);
SEXP savvy_RwasmtimeNativeRuntime_compile_core__ffi(SEXP self__, SEXP c_arg__module);
SEXP savvy_RwasmtimeNativeRuntime_component_exports__ffi(SEXP self__, SEXP c_arg__component);
SEXP savvy_RwasmtimeNativeRuntime_component_imports__ffi(SEXP self__, SEXP c_arg__component);
SEXP savvy_RwasmtimeNativeRuntime_deserialize_core__ffi(SEXP self__, SEXP c_arg__bytes);
SEXP savvy_RwasmtimeNativeRuntime_instantiate_core__ffi(SEXP self__, SEXP c_arg__module, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms);
SEXP savvy_RwasmtimeNativeRuntime_run_wasi_p1__ffi(SEXP self__, SEXP c_arg__module, SEXP c_arg__args, SEXP c_arg__env_names, SEXP c_arg__env_values, SEXP c_arg__preopen_guest, SEXP c_arg__preopen_host, SEXP c_arg__preopen_readonly, SEXP c_arg__stdin, SEXP c_arg__stdout, SEXP c_arg__stderr, SEXP c_arg__memory_bytes, SEXP c_arg__table_elements, SEXP c_arg__instances, SEXP c_arg__fuel, SEXP c_arg__wall_time_ms, SEXP c_arg__input);
