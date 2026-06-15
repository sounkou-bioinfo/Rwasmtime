use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use rwasmtime::app::RwasmtimeError;
use rwasmtime::backend::{
    CoreExecutionLimits, CoreHostFunc, CoreInstance, CoreItem, CoreModule, WasmtimeRuntime,
};
use rwasmtime::config::OptLevel;
use rwasmtime::{CompilerSpec, CompilerStrategy, FeatureSpec, RuntimeSpec, StdioMode, WasiSpec};
use savvy::savvy;
use savvy::{
    FunctionArgs, FunctionSexp, ListSexp, NullSexp, OwnedIntegerSexp, OwnedListSexp,
    OwnedLogicalSexp, OwnedRawSexp, OwnedRealSexp, OwnedStringSexp, RawSexp, Sexp, StringSexp,
    TypedSexp,
};

/// Native Wasmtime runtime handle owned by the Savvy adapter.
/// @noRd
#[savvy]
pub struct RwasmtimeNativeRuntime {
    inner: WasmtimeRuntime,
}

/// Compiled core Wasm module owned by the Savvy adapter.
/// @noRd
#[savvy]
pub struct RwasmtimeNativeModule {
    inner: CoreModule,
}

/// Persistent core Wasm instance owned by the Savvy adapter.
/// @noRd
#[savvy]
pub struct RwasmtimeNativeInstance {
    inner: CoreInstance,
}

#[savvy]
impl RwasmtimeNativeRuntime {
    /// Build a native Wasmtime runtime from copied R-side options.
    fn build(
        compiler_strategy: &str,
        opt_level: &str,
        parallel: bool,
        component_model: bool,
        component_model_async: bool,
        simd: bool,
        relaxed_simd: bool,
        relaxed_simd_deterministic: bool,
        bulk_memory: bool,
        multi_memory: bool,
        memory64: bool,
        threads: bool,
        exceptions: bool,
        legacy_exceptions: bool,
        gc: bool,
    ) -> savvy::Result<Self> {
        let compiler = CompilerSpec {
            strategy: parse_compiler_strategy(compiler_strategy)?,
            opt_level: parse_opt_level(opt_level)?,
            parallel,
        };
        let mut features = FeatureSpec::new();
        features.component_model = component_model;
        features.component_model_async = component_model_async;
        features.simd = simd;
        features.relaxed_simd = relaxed_simd;
        features.relaxed_simd_deterministic = relaxed_simd_deterministic;
        features.bulk_memory = bulk_memory;
        features.multi_memory = multi_memory;
        features.memory64 = memory64;
        features.threads = threads;
        features.exceptions = exceptions;
        features.legacy_exceptions = legacy_exceptions;
        features.gc = gc;
        let runtime = RuntimeSpec::new()
            .compiler(compiler)
            .features(features)
            .build_wasmtime()
            .map_err(to_savvy_error)?;
        Ok(Self { inner: runtime })
    }

    /// Compile a core Wasm module once for later instantiation.
    fn compile_core(&self, module: Sexp) -> savvy::Result<RwasmtimeNativeModule> {
        let bytes;
        let module_ref: &[u8] = match module.into_typed() {
            TypedSexp::Raw(value) => {
                bytes = value.to_vec();
                &bytes
            }
            TypedSexp::String(value) => {
                let text = single_string(value, "module")?;
                bytes = text.into_bytes();
                &bytes
            }
            _ => {
                return Err(savvy::Error::new(
                    "module must be a character scalar or raw vector",
                ))
            }
        };
        let inner = self
            .inner
            .compile_core(module_ref)
            .map_err(to_savvy_error)?;
        Ok(RwasmtimeNativeModule { inner })
    }

    /// Deserialize a previously serialized core Wasm module artifact.
    fn deserialize_core(&self, bytes: Sexp) -> savvy::Result<RwasmtimeNativeModule> {
        let bytes = raw_vec(bytes, "bytes")?;
        let inner = self
            .inner
            .deserialize_core(&bytes)
            .map_err(to_savvy_error)?;
        Ok(RwasmtimeNativeModule { inner })
    }

    /// Compile a Wasm component and return copied import metadata.
    fn component_imports(&self, component: &str) -> savvy::Result<Sexp> {
        let items = self
            .inner
            .component_imports(component)
            .map_err(to_savvy_error)?;
        component_items_value(items)
    }

    /// Compile a Wasm component and return copied export metadata.
    fn component_exports(&self, component: &str) -> savvy::Result<Sexp> {
        let items = self
            .inner
            .component_exports(component)
            .map_err(to_savvy_error)?;
        component_items_value(items)
    }

    /// Instantiate a core Wasm module once and keep its store/instance alive.
    fn instantiate_core(
        &self,
        module: &str,
        memory_bytes: f64,
        table_elements: f64,
        instances: f64,
        fuel: f64,
        wall_time_ms: f64,
    ) -> savvy::Result<RwasmtimeNativeInstance> {
        let limits =
            core_execution_limits(memory_bytes, table_elements, instances, fuel, wall_time_ms)?;
        let inner = self
            .inner
            .instantiate_core(module, limits)
            .map_err(to_savvy_error)?;
        Ok(RwasmtimeNativeInstance { inner })
    }

    /// Call a core Wasm export using Wasmtime-discovered dynamic signatures.
    fn call_core(
        &self,
        module: &str,
        export: &str,
        args: ListSexp,
        memory_bytes: f64,
        table_elements: f64,
        instances: f64,
        fuel: f64,
        wall_time_ms: f64,
    ) -> savvy::Result<Sexp> {
        let limits =
            core_execution_limits(memory_bytes, table_elements, instances, fuel, wall_time_ms)?;
        call_core_dynamic(&self.inner, module, export, args, limits)
    }

    /// Run a WASIp1 command module and return captured stdout/stderr bytes.
    fn run_wasi_p1(
        &self,
        module: &str,
        args: StringSexp,
        env_names: StringSexp,
        env_values: StringSexp,
        preopen_guest: StringSexp,
        preopen_host: StringSexp,
        preopen_readonly: Sexp,
        stdin: &str,
        stdout: &str,
        stderr: &str,
        memory_bytes: f64,
        table_elements: f64,
        instances: f64,
        fuel: f64,
        wall_time_ms: f64,
        input: Sexp,
    ) -> savvy::Result<Sexp> {
        let wasi = wasi_spec_from_parts(
            args,
            env_names,
            env_values,
            preopen_guest,
            preopen_host,
            preopen_readonly,
            stdin,
            stdout,
            stderr,
            input,
        )?;
        let limits =
            core_execution_limits(memory_bytes, table_elements, instances, fuel, wall_time_ms)?;
        let output = self
            .inner
            .run_wasi_p1_command(module, &wasi, limits)
            .map_err(to_savvy_error)?;
        wasi_output_value(output.stdout, output.stderr)
    }
}

#[savvy]
impl RwasmtimeNativeModule {
    /// Instantiate this compiled core module as a fresh persistent instance.
    fn instantiate(
        &self,
        memory_bytes: f64,
        table_elements: f64,
        instances: f64,
        fuel: f64,
        wall_time_ms: f64,
    ) -> savvy::Result<RwasmtimeNativeInstance> {
        let limits =
            core_execution_limits(memory_bytes, table_elements, instances, fuel, wall_time_ms)?;
        let inner = self.inner.instantiate(limits).map_err(to_savvy_error)?;
        Ok(RwasmtimeNativeInstance { inner })
    }

    /// Instantiate this compiled core module with R-backed core callback imports linked.
    fn instantiate_callbacks(
        &self,
        callback_modules: StringSexp,
        callback_names: StringSexp,
        callback_abis: StringSexp,
        callback_functions: ListSexp,
        memory_bytes: f64,
        table_elements: f64,
        instances: f64,
        fuel: f64,
        wall_time_ms: f64,
    ) -> savvy::Result<RwasmtimeNativeInstance> {
        let host_funcs = build_core_host_funcs(
            &self.inner,
            callback_modules,
            callback_names,
            callback_abis,
            callback_functions,
            false,
        )?;
        let limits =
            core_execution_limits(memory_bytes, table_elements, instances, fuel, wall_time_ms)?;
        let inner = self
            .inner
            .instantiate_with_host_funcs(host_funcs, limits)
            .map_err(to_savvy_error)?;
        Ok(RwasmtimeNativeInstance { inner })
    }

    /// Instantiate this compiled core module with WASIp1 imports linked.
    fn instantiate_wasi_p1(
        &self,
        args: StringSexp,
        env_names: StringSexp,
        env_values: StringSexp,
        preopen_guest: StringSexp,
        preopen_host: StringSexp,
        preopen_readonly: Sexp,
        stdin: &str,
        stdout: &str,
        stderr: &str,
        memory_bytes: f64,
        table_elements: f64,
        instances: f64,
        fuel: f64,
        wall_time_ms: f64,
        input: Sexp,
    ) -> savvy::Result<RwasmtimeNativeInstance> {
        let wasi = wasi_spec_from_parts(
            args,
            env_names,
            env_values,
            preopen_guest,
            preopen_host,
            preopen_readonly,
            stdin,
            stdout,
            stderr,
            input,
        )?;
        let limits =
            core_execution_limits(memory_bytes, table_elements, instances, fuel, wall_time_ms)?;
        let inner = self
            .inner
            .instantiate_wasi_p1(&wasi, limits)
            .map_err(to_savvy_error)?;
        Ok(RwasmtimeNativeInstance { inner })
    }

    /// Instantiate this compiled core module with WASIp1 and R-backed callback imports linked.
    fn instantiate_wasi_p1_callbacks(
        &self,
        callback_modules: StringSexp,
        callback_names: StringSexp,
        callback_abis: StringSexp,
        callback_functions: ListSexp,
        args: StringSexp,
        env_names: StringSexp,
        env_values: StringSexp,
        preopen_guest: StringSexp,
        preopen_host: StringSexp,
        preopen_readonly: Sexp,
        stdin: &str,
        stdout: &str,
        stderr: &str,
        memory_bytes: f64,
        table_elements: f64,
        instances: f64,
        fuel: f64,
        wall_time_ms: f64,
        input: Sexp,
    ) -> savvy::Result<RwasmtimeNativeInstance> {
        let host_funcs = build_core_host_funcs(
            &self.inner,
            callback_modules,
            callback_names,
            callback_abis,
            callback_functions,
            true,
        )?;
        let wasi = wasi_spec_from_parts(
            args,
            env_names,
            env_values,
            preopen_guest,
            preopen_host,
            preopen_readonly,
            stdin,
            stdout,
            stderr,
            input,
        )?;
        let limits =
            core_execution_limits(memory_bytes, table_elements, instances, fuel, wall_time_ms)?;
        let inner = self
            .inner
            .instantiate_wasi_p1_with_host_funcs(&wasi, host_funcs, limits)
            .map_err(to_savvy_error)?;
        Ok(RwasmtimeNativeInstance { inner })
    }

    /// Return copied core import metadata for this compiled module.
    fn imports(&self) -> savvy::Result<Sexp> {
        core_items_value(self.inner.imports())
    }

    /// Return copied core export metadata for this compiled module.
    fn exports(&self) -> savvy::Result<Sexp> {
        core_items_value(self.inner.exports())
    }

    /// Serialize this compiled core module to bytes for AOT save/load.
    fn serialize(&self) -> savvy::Result<Sexp> {
        let bytes = self.inner.serialize().map_err(to_savvy_error)?;
        raw_value(bytes)?.into()
    }
}

#[savvy]
impl RwasmtimeNativeInstance {
    /// Call a core Wasm export on this persistent instance.
    fn call_core(&mut self, export: &str, args: ListSexp) -> savvy::Result<Sexp> {
        call_core_instance_dynamic(&mut self.inner, export, args)
    }

    /// Return the current size of an exported linear memory, in Wasm pages.
    fn memory_size(&mut self, name: &str) -> savvy::Result<Sexp> {
        let pages = self.inner.memory_size_pages(name).map_err(to_savvy_error)?;
        real_scalar(pages as f64)?.into()
    }

    /// Grow an exported linear memory by a number of Wasm pages; returns the previous size.
    fn memory_grow(&mut self, name: &str, pages: f64) -> savvy::Result<Sexp> {
        let pages = whole_nonnegative_u64(pages, "pages")?;
        let previous = self
            .inner
            .memory_grow_pages(name, pages)
            .map_err(to_savvy_error)?;
        real_scalar(previous as f64)?.into()
    }

    /// Copy bytes out of an exported linear memory.
    fn memory_read(&mut self, name: &str, offset: f64, len: f64) -> savvy::Result<Sexp> {
        let offset = whole_nonnegative_usize(offset, "offset")?;
        let len = whole_nonnegative_usize(len, "length")?;
        let bytes = self
            .inner
            .memory_read(name, offset, len)
            .map_err(to_savvy_error)?;
        raw_value(bytes)?.into()
    }

    /// Copy raw bytes into an exported linear memory.
    fn memory_write(&mut self, name: &str, offset: f64, value: Sexp) -> savvy::Result<Sexp> {
        let offset = whole_nonnegative_usize(offset, "offset")?;
        let bytes = raw_vec(value, "value")?;
        self.inner
            .memory_write(name, offset, &bytes)
            .map_err(to_savvy_error)?;
        Ok(NullSexp.into())
    }

    /// Return captured WASIp1 stdout/stderr bytes for this instance.
    fn wasi_output(&self) -> savvy::Result<Sexp> {
        let output = self.inner.wasi_output();
        wasi_output_value(output.stdout, output.stderr)
    }
}

/// Return whether the generated Savvy adapter is linked to the native backend.
/// @noRd
#[savvy]
fn rwasmtime_backend_status() -> savvy::Result<Sexp> {
    str_scalar("native")?.into()
}

struct PreservedRFunction {
    inner: savvy::ffi::SEXP,
    token: savvy::ffi::SEXP,
}

// Wasmtime requires host functions to be Send + Sync + 'static. The Savvy
// adapter only uses this wrapper for synchronous calls initiated from R on the R
// main thread. It must not be used for async worker-thread callback servicing;
// that future path needs the callback broker described in the R API docs.
unsafe impl Send for PreservedRFunction {}
unsafe impl Sync for PreservedRFunction {}

impl PreservedRFunction {
    fn new(value: Sexp) -> savvy::Result<Self> {
        let function = FunctionSexp::try_from(value)?;
        let inner = function.inner();
        let token = savvy::protect::insert_to_preserved_list(inner);
        Ok(Self { inner, token })
    }

    fn call_core(
        &self,
        name: &str,
        args: &[wasmtime::Val],
        result_tys: &[wasmtime::ValType],
        results: &mut [wasmtime::Val],
    ) -> wasmtime::Result<()> {
        let mut fargs = FunctionArgs::new();
        for arg in args.iter() {
            let value = val_to_sexp(arg.clone()).map_err(savvy_to_wasmtime_error)?;
            fargs.add("", value).map_err(savvy_to_wasmtime_error)?;
        }
        let out = FunctionSexp(self.inner)
            .call(fargs)
            .map_err(|err| wasmtime_error(format!("R callback `{name}` failed: {err}")))?;
        assign_callback_results(name, Sexp::from(out), result_tys, results)
            .map_err(savvy_to_wasmtime_error)
    }

    fn call_core_memory_request(
        &self,
        callback_name: &str,
        request_name: &[u8],
        payload: &[u8],
        result_cap: u32,
    ) -> wasmtime::Result<Vec<u8>> {
        let request_name = std::str::from_utf8(request_name).map_err(|err| {
            wasmtime_error(format!(
                "R callback `{callback_name}` received non-UTF-8 request name: {err}"
            ))
        })?;
        let result_cap = i32::try_from(result_cap).map_err(|_| {
            wasmtime_error(format!(
                "R callback `{callback_name}` received result_cap larger than i32::MAX"
            ))
        })?;
        let mut fargs = FunctionArgs::new();
        fargs
            .add(
                "name",
                str_scalar(request_name).map_err(savvy_to_wasmtime_error)?,
            )
            .map_err(savvy_to_wasmtime_error)?;
        fargs
            .add(
                "payload",
                raw_value(payload.to_vec()).map_err(savvy_to_wasmtime_error)?,
            )
            .map_err(savvy_to_wasmtime_error)?;
        fargs
            .add(
                "result_cap",
                int_scalar(result_cap).map_err(savvy_to_wasmtime_error)?,
            )
            .map_err(savvy_to_wasmtime_error)?;
        let out = FunctionSexp(self.inner)
            .call(fargs)
            .map_err(|err| wasmtime_error(format!("R callback `{callback_name}` failed: {err}")))?;
        memory_request_response_bytes(callback_name, Sexp::from(out))
            .map_err(savvy_to_wasmtime_error)
    }
}

impl Drop for PreservedRFunction {
    fn drop(&mut self) {
        savvy::protect::release_from_preserved_list(self.token);
    }
}

fn build_core_host_funcs(
    module: &CoreModule,
    callback_modules: StringSexp,
    callback_names: StringSexp,
    callback_abis: StringSexp,
    callback_functions: ListSexp,
    allow_unprovided_imports: bool,
) -> savvy::Result<Vec<CoreHostFunc>> {
    let modules = string_vec(callback_modules);
    let names = string_vec(callback_names);
    let abis = string_vec(callback_abis);
    if modules.len() != names.len()
        || names.len() != abis.len()
        || names.len() != callback_functions.len()
    {
        return Err(savvy::Error::new(
            "callback module/name/abi/function vectors must have the same length",
        ));
    }

    let mut callbacks = HashMap::new();
    for index in 0..names.len() {
        let key = format!("{}::{}", modules[index], names[index]);
        if callbacks.contains_key(&key) {
            return Err(savvy::Error::new(&format!(
                "duplicate callback import: {key}"
            )));
        }
        let abi = abis[index].clone();
        if abi != "core" && abi != "core_memory_request" {
            return Err(savvy::Error::new(&format!(
                "unsupported native core callback ABI for `{key}`: {abi}"
            )));
        }
        let function = callback_functions.get_by_index(index).ok_or_else(|| {
            savvy::Error::new("callback function list is shorter than callback names")
        })?;
        callbacks.insert(key, (Arc::new(PreservedRFunction::new(function)?), abi));
    }

    let mut used = HashSet::new();
    let mut host_funcs = Vec::new();
    let mut linked_signatures: HashMap<String, (String, Vec<String>, Vec<String>)> = HashMap::new();
    for import in module.func_imports() {
        let key = format!("{}::{}", import.module, import.name);
        let Some((function, abi)) = callbacks.get(&key).cloned() else {
            if allow_unprovided_imports {
                continue;
            }
            return Err(savvy::Error::new(&format!(
                "missing R callback for core import `{key}`"
            )));
        };
        used.insert(key.clone());
        let signature = (
            abi.clone(),
            val_type_names(&import.params),
            val_type_names(&import.results),
        );
        if let Some(existing) = linked_signatures.get(&key) {
            if existing != &signature {
                return Err(savvy::Error::new(&format!(
                    "core callback import `{key}` is used with incompatible function signatures or callback ABIs"
                )));
            }
            continue;
        }
        linked_signatures.insert(key.clone(), signature);
        if abi == "core_memory_request" {
            let expected_params = vec!["i32".to_string(); 6];
            let expected_results = vec!["i32".to_string()];
            if val_type_names(&import.params) != expected_params
                || val_type_names(&import.results) != expected_results
            {
                return Err(savvy::Error::new(&format!(
                    "core_memory_request callback `{key}` requires signature (i32, i32, i32, i32, i32, i32) -> i32"
                )));
            }
            host_funcs.push(CoreHostFunc::new_memory_request(
                import.module,
                import.name,
                Arc::new(move |callback_name, request_name, payload, result_cap| {
                    function.call_core_memory_request(
                        callback_name,
                        request_name,
                        payload,
                        result_cap,
                    )
                }),
            ));
            continue;
        }
        let result_tys = import.results.clone();
        host_funcs.push(CoreHostFunc::new(
            import.module,
            import.name,
            import.params,
            import.results,
            Arc::new(move |name, args, results| {
                function.call_core(name, args, &result_tys, results)
            }),
        ));
    }

    let mut unused: Vec<_> = callbacks
        .keys()
        .filter(|key| !used.contains(*key))
        .cloned()
        .collect();
    unused.sort();
    if !unused.is_empty() {
        return Err(savvy::Error::new(&format!(
            "R callback(s) were provided but not imported by the module: {}",
            unused.join(", ")
        )));
    }
    Ok(host_funcs)
}

fn val_type_names(types: &[wasmtime::ValType]) -> Vec<String> {
    types.iter().map(|ty| ty.to_string()).collect()
}

fn memory_request_response_bytes(name: &str, value: Sexp) -> savvy::Result<Vec<u8>> {
    if value.is_raw() {
        return Ok(RawSexp::try_from(value)?.to_vec());
    }
    if value.is_string() {
        let strings = StringSexp::try_from(value)?;
        let values: Vec<String> = strings.to_vec().into_iter().map(String::from).collect();
        if values.len() != 1 {
            return Err(savvy::Error::new(&format!(
                "R callback `{name}` must return a raw vector or character scalar"
            )));
        }
        return Ok(values[0].as_bytes().to_vec());
    }
    Err(savvy::Error::new(&format!(
        "R callback `{name}` must return a raw vector or character scalar"
    )))
}

fn assign_callback_results(
    name: &str,
    value: Sexp,
    result_tys: &[wasmtime::ValType],
    results: &mut [wasmtime::Val],
) -> savvy::Result<()> {
    if result_tys.is_empty() {
        return Ok(());
    }
    if result_tys.len() == 1 {
        results[0] = sexp_to_val(value, &result_tys[0], 1)?;
        return Ok(());
    }
    let list = ListSexp::try_from(value).map_err(|_| {
        savvy::Error::new(&format!(
            "R callback `{name}` must return a list for {} Wasm results",
            result_tys.len()
        ))
    })?;
    if list.len() != result_tys.len() {
        return Err(savvy::Error::new(&format!(
            "R callback `{name}` returned {} value(s), but Wasm expects {} result(s)",
            list.len(),
            result_tys.len()
        )));
    }
    for (index, expected) in result_tys.iter().enumerate() {
        let value = list
            .get_by_index(index)
            .ok_or_else(|| savvy::Error::new("callback result list is shorter than expected"))?;
        results[index] = sexp_to_val(value, expected, index + 1)?;
    }
    Ok(())
}

fn wasmtime_error(message: impl Into<String>) -> wasmtime::Error {
    wasmtime::Error::msg(message.into())
}

fn savvy_to_wasmtime_error(err: savvy::Error) -> wasmtime::Error {
    wasmtime_error(err.to_string())
}

fn call_core_dynamic(
    runtime: &WasmtimeRuntime,
    module: &str,
    export: &str,
    args: ListSexp,
    limits: CoreExecutionLimits,
) -> savvy::Result<Sexp> {
    let mut instance = runtime
        .instantiate_core(module, limits)
        .map_err(to_savvy_error)?;
    call_core_instance_dynamic(&mut instance, export, args)
}

fn call_core_instance_dynamic(
    instance: &mut CoreInstance,
    export: &str,
    args: ListSexp,
) -> savvy::Result<Sexp> {
    let ty = instance.func_type(export).map_err(to_savvy_error)?;
    let params: Vec<_> = ty.params().collect();
    let result_tys: Vec<_> = ty.results().collect();
    if params.len() != args.len() {
        return Err(savvy::Error::new(&format!(
            "export `{export}` expects {} argument(s), got {}",
            params.len(),
            args.len()
        )));
    }

    let mut wasm_args = Vec::with_capacity(args.len());
    for (index, ((_, value), expected)) in args.iter().zip(params.iter()).enumerate() {
        wasm_args.push(sexp_to_val(value, expected, index + 1)?);
    }
    let mut wasm_results = result_tys
        .iter()
        .map(default_val_for_type)
        .collect::<savvy::Result<Vec<_>>>()?;
    instance
        .call_export(export, &wasm_args, &mut wasm_results)
        .map_err(to_savvy_error)?;

    values_to_sexp(wasm_results)
}

fn sexp_to_val(
    value: Sexp,
    expected: &wasmtime::ValType,
    index: usize,
) -> savvy::Result<wasmtime::Val> {
    match expected {
        wasmtime::ValType::I32 => Ok(wasmtime::Val::I32(coerce_i32(value, index)?)),
        wasmtime::ValType::I64 => Ok(wasmtime::Val::I64(coerce_i64(value, index)?)),
        wasmtime::ValType::F32 => Ok(wasmtime::Val::F32(coerce_f32(value, index)?.to_bits())),
        wasmtime::ValType::F64 => Ok(wasmtime::Val::F64(coerce_f64(value, index)?.to_bits())),
        wasmtime::ValType::V128 => Ok(wasmtime::Val::V128(coerce_v128(value, index)?)),
        wasmtime::ValType::Ref(ref_ty) => coerce_null_ref(value, ref_ty, index),
    }
}

fn default_val_for_type(ty: &wasmtime::ValType) -> savvy::Result<wasmtime::Val> {
    wasmtime::Val::default_for_ty(ty).ok_or_else(|| {
        savvy::Error::new(&format!(
            "core result value type {ty:?} has no default value for dynamic calls"
        ))
    })
}

fn values_to_sexp(values: Vec<wasmtime::Val>) -> savvy::Result<Sexp> {
    if values.is_empty() {
        return Ok(NullSexp.into());
    }
    if values.len() == 1 {
        return val_to_sexp(values.into_iter().next().unwrap());
    }
    let mut out = OwnedListSexp::new(values.len(), false)?;
    for (i, value) in values.into_iter().enumerate() {
        out.set_value(i, val_to_sexp(value)?)?;
    }
    out.into()
}

fn val_to_sexp(value: wasmtime::Val) -> savvy::Result<Sexp> {
    match value {
        wasmtime::Val::I32(value) => int_scalar(value)?.into(),
        wasmtime::Val::I64(value) => str_scalar(&value.to_string())?.into(),
        wasmtime::Val::F32(bits) => real_scalar(f32::from_bits(bits) as f64)?.into(),
        wasmtime::Val::F64(bits) => real_scalar(f64::from_bits(bits))?.into(),
        wasmtime::Val::V128(value) => raw_value(value.as_u128().to_le_bytes().to_vec())?.into(),
        wasmtime::Val::FuncRef(None) => null_ref_value("funcref"),
        wasmtime::Val::ExternRef(None) => null_ref_value("externref"),
        wasmtime::Val::AnyRef(None) => null_ref_value("anyref"),
        wasmtime::Val::ExnRef(None) => null_ref_value("exnref"),
        wasmtime::Val::ContRef(None) => null_ref_value("contref"),
        other => Err(savvy::Error::new(&format!(
            "non-null core result reference {other:?} is not implemented"
        ))),
    }
}

fn coerce_i32(value: Sexp, index: usize) -> savvy::Result<i32> {
    match value.into_typed() {
        TypedSexp::Integer(value) if value.len() == 1 => Ok(value.as_slice()[0]),
        TypedSexp::Logical(value) if value.len() == 1 => {
            Ok(if value.iter().next().unwrap_or(false) {
                1
            } else {
                0
            })
        }
        TypedSexp::Real(value) if value.len() == 1 => checked_integral(
            value.as_slice()[0],
            i32::MIN as f64,
            i32::MAX as f64,
            "i32",
            index,
        )
        .map(|v| v as i32),
        _ => Err(numeric_error("i32", index)),
    }
}

fn coerce_i64(value: Sexp, index: usize) -> savvy::Result<i64> {
    match value.into_typed() {
        TypedSexp::Integer(value) if value.len() == 1 => Ok(value.as_slice()[0] as i64),
        TypedSexp::Logical(value) if value.len() == 1 => {
            Ok(if value.iter().next().unwrap_or(false) {
                1
            } else {
                0
            })
        }
        TypedSexp::Real(value) if value.len() == 1 => checked_safe_i64(value.as_slice()[0], index),
        TypedSexp::String(value) if value.len() == 1 => value
            .iter()
            .next()
            .ok_or_else(|| numeric_error("i64", index))?
            .parse::<i64>()
            .map_err(|_| numeric_error("i64", index)),
        _ => Err(numeric_error("i64", index)),
    }
}

fn coerce_v128(value: Sexp, index: usize) -> savvy::Result<wasmtime::V128> {
    let bytes = raw_vec(value, &format!("argument {index}"))?;
    if bytes.len() != 16 {
        return Err(savvy::Error::new(&format!(
            "argument {index} for Wasm v128 must be a raw vector of length 16"
        )));
    }
    let mut array = [0_u8; 16];
    array.copy_from_slice(&bytes);
    Ok(wasmtime::V128::from(u128::from_le_bytes(array)))
}

fn coerce_null_ref(
    value: Sexp,
    ref_ty: &wasmtime::RefType,
    index: usize,
) -> savvy::Result<wasmtime::Val> {
    match value.into_typed() {
        TypedSexp::Null(_) if ref_ty.is_nullable() => {
            Ok(wasmtime::Val::null_ref(ref_ty.heap_type()))
        }
        TypedSexp::Null(_) => Err(savvy::Error::new(&format!(
            "argument {index} is NULL but the expected reference type is non-nullable"
        ))),
        _ => Err(savvy::Error::new(&format!(
            "only NULL is implemented for core reference argument {index}"
        ))),
    }
}

fn coerce_f32(value: Sexp, index: usize) -> savvy::Result<f32> {
    let value = coerce_f64(value, index)?;
    if value.is_finite() && value >= f32::MIN as f64 && value <= f32::MAX as f64 {
        Ok(value as f32)
    } else {
        Err(numeric_error("f32", index))
    }
}

fn coerce_f64(value: Sexp, index: usize) -> savvy::Result<f64> {
    match value.into_typed() {
        TypedSexp::Integer(value) if value.len() == 1 => Ok(value.as_slice()[0] as f64),
        TypedSexp::Logical(value) if value.len() == 1 => {
            Ok(if value.iter().next().unwrap_or(false) {
                1.0
            } else {
                0.0
            })
        }
        TypedSexp::Real(value) if value.len() == 1 && value.as_slice()[0].is_finite() => {
            Ok(value.as_slice()[0])
        }
        _ => Err(numeric_error("f64", index)),
    }
}

fn checked_integral(
    value: f64,
    min: f64,
    max: f64,
    expected: &str,
    index: usize,
) -> savvy::Result<f64> {
    if value.is_finite() && value.fract() == 0.0 && value >= min && value <= max {
        Ok(value)
    } else {
        Err(numeric_error(expected, index))
    }
}

fn checked_safe_i64(value: f64, index: usize) -> savvy::Result<i64> {
    const MAX_EXACT_DOUBLE_INTEGER: f64 = 9_007_199_254_740_992.0;
    if value.is_finite() && value.fract() == 0.0 && value.abs() < MAX_EXACT_DOUBLE_INTEGER {
        Ok(value as i64)
    } else {
        Err(savvy::Error::new(&format!(
            "argument {index} cannot be represented exactly as Wasm i64; use a decimal string for values at or above 2^53"
        )))
    }
}

fn numeric_error(expected: &str, index: usize) -> savvy::Error {
    savvy::Error::new(&format!(
        "argument {index} cannot be represented as Wasm {expected}"
    ))
}

fn parse_compiler_strategy(value: &str) -> savvy::Result<CompilerStrategy> {
    match value {
        "auto" => Ok(CompilerStrategy::Auto),
        "cranelift" => Ok(CompilerStrategy::Cranelift),
        "winch" => Ok(CompilerStrategy::Winch),
        other => Err(savvy::Error::new(&format!(
            "unsupported compiler strategy `{other}`"
        ))),
    }
}

fn parse_opt_level(value: &str) -> savvy::Result<OptLevel> {
    match value {
        "none" => Ok(OptLevel::None),
        "speed" => Ok(OptLevel::Speed),
        "speed_and_size" => Ok(OptLevel::SpeedAndSize),
        other => Err(savvy::Error::new(&format!(
            "unsupported opt_level `{other}`"
        ))),
    }
}

fn wasi_spec_from_parts(
    args: StringSexp,
    env_names: StringSexp,
    env_values: StringSexp,
    preopen_guest: StringSexp,
    preopen_host: StringSexp,
    preopen_readonly: Sexp,
    stdin: &str,
    stdout: &str,
    stderr: &str,
    input: Sexp,
) -> savvy::Result<WasiSpec> {
    let mut wasi = WasiSpec::new();
    for arg in string_vec(args) {
        wasi = wasi.arg(arg);
    }

    let env_names = string_vec(env_names);
    let env_values = string_vec(env_values);
    if env_names.len() != env_values.len() {
        return Err(savvy::Error::new(
            "WASI env names and values must have the same length",
        ));
    }
    for (name, value) in env_names.into_iter().zip(env_values.into_iter()) {
        wasi = wasi.env(name, value);
    }

    let preopen_guest = string_vec(preopen_guest);
    let preopen_host = string_vec(preopen_host);
    let preopen_readonly = logical_vec(preopen_readonly, "preopen_readonly")?;
    if preopen_guest.len() != preopen_host.len() || preopen_guest.len() != preopen_readonly.len() {
        return Err(savvy::Error::new(
            "WASI preopen vectors must have the same length",
        ));
    }
    for ((guest, host), readonly) in preopen_guest
        .into_iter()
        .zip(preopen_host.into_iter())
        .zip(preopen_readonly.into_iter())
    {
        wasi = wasi.preopen(guest, host, readonly);
    }

    wasi = wasi.stdio(
        parse_stdio(stdin, "stdin")?,
        parse_stdio(stdout, "stdout")?,
        parse_stdio(stderr, "stderr")?,
    );
    if let Some(input) = optional_raw_vec(input, "input")? {
        wasi = wasi.stdin_bytes(input);
    }
    Ok(wasi)
}

fn wasi_output_value(stdout: Vec<u8>, stderr: Vec<u8>) -> savvy::Result<Sexp> {
    let mut out = OwnedListSexp::new(2, true)?;
    out.set_name_and_value(0, "stdout_raw", raw_value(stdout)?)?;
    out.set_name_and_value(1, "stderr_raw", raw_value(stderr)?)?;
    out.into()
}

fn parse_stdio(value: &str, name: &str) -> savvy::Result<StdioMode> {
    match value {
        "empty" => Ok(StdioMode::Empty),
        "inherit" => Ok(StdioMode::Inherit),
        "string" => Ok(StdioMode::String),
        "file" => Ok(StdioMode::File),
        "capture" => Ok(StdioMode::Capture),
        "discard" => Ok(StdioMode::Discard),
        other => Err(savvy::Error::new(&format!(
            "unsupported WASI {name} stdio mode `{other}`"
        ))),
    }
}

fn string_vec(value: StringSexp) -> Vec<String> {
    value.iter().map(|item| item.to_string()).collect()
}

fn single_string(value: StringSexp, name: &str) -> savvy::Result<String> {
    let values: Vec<String> = string_vec(value);
    if values.len() != 1 {
        return Err(savvy::Error::new(&format!(
            "{name} must be a character scalar"
        )));
    }
    Ok(values[0].clone())
}

fn logical_vec(value: Sexp, name: &str) -> savvy::Result<Vec<bool>> {
    match value.into_typed() {
        TypedSexp::Logical(value) => Ok(value.iter().collect()),
        _ => Err(savvy::Error::new(&format!(
            "{name} must be a logical vector"
        ))),
    }
}

fn raw_value(value: Vec<u8>) -> savvy::Result<OwnedRawSexp> {
    OwnedRawSexp::try_from(value)
}

fn raw_vec(value: Sexp, name: &str) -> savvy::Result<Vec<u8>> {
    match value.into_typed() {
        TypedSexp::Raw(value) => Ok(value.to_vec()),
        _ => Err(savvy::Error::new(&format!("{name} must be a raw vector"))),
    }
}

fn optional_raw_vec(value: Sexp, name: &str) -> savvy::Result<Option<Vec<u8>>> {
    match value.into_typed() {
        TypedSexp::Null(_) => Ok(None),
        TypedSexp::Raw(value) => Ok(Some(value.to_vec())),
        _ => Err(savvy::Error::new(&format!(
            "{name} must be NULL or a raw vector"
        ))),
    }
}

fn whole_nonnegative_usize(value: f64, name: &str) -> savvy::Result<usize> {
    if value.is_finite() && value.fract() == 0.0 && value >= 0.0 && value <= usize::MAX as f64 {
        Ok(value as usize)
    } else {
        Err(savvy::Error::new(&format!(
            "{name} must be a non-negative whole number representable as usize"
        )))
    }
}

fn whole_nonnegative_u64(value: f64, name: &str) -> savvy::Result<u64> {
    if value.is_finite() && value.fract() == 0.0 && value >= 0.0 && value <= u64::MAX as f64 {
        Ok(value as u64)
    } else {
        Err(savvy::Error::new(&format!(
            "{name} must be a non-negative whole number representable as u64"
        )))
    }
}

fn optional_whole_nonnegative_u64(value: f64, name: &str) -> savvy::Result<Option<u64>> {
    if value < 0.0 {
        Ok(None)
    } else {
        whole_nonnegative_u64(value, name).map(Some)
    }
}

fn core_execution_limits(
    memory_bytes: f64,
    table_elements: f64,
    instances: f64,
    fuel: f64,
    wall_time_ms: f64,
) -> savvy::Result<CoreExecutionLimits> {
    Ok(CoreExecutionLimits::new(
        optional_whole_nonnegative_u64(fuel, "fuel")?,
        optional_whole_nonnegative_u64(wall_time_ms, "wall_time_ms")?,
    )
    .resource_limits(
        optional_whole_nonnegative_u64(memory_bytes, "memory_bytes")?,
        optional_whole_nonnegative_u64(table_elements, "table_elements")?,
        optional_whole_nonnegative_u64(instances, "instances")?,
    ))
}

fn component_items_value(items: Vec<rwasmtime::component::ComponentItem>) -> savvy::Result<Sexp> {
    let mut out = OwnedListSexp::new(items.len(), false)?;
    for (i, item) in items.into_iter().enumerate() {
        out.set_value(i, component_item_value(item)?)?;
    }
    out.into()
}

fn core_items_value(items: Vec<CoreItem>) -> savvy::Result<Sexp> {
    let mut out = OwnedListSexp::new(items.len(), false)?;
    for (i, item) in items.into_iter().enumerate() {
        out.set_value(i, core_item_value(item)?)?;
    }
    out.into()
}

fn core_item_value(item: CoreItem) -> savvy::Result<Sexp> {
    let mut out = OwnedListSexp::new(13, true)?;
    out.set_name_and_value(0, "module", optional_str_value(item.module.as_deref())?)?;
    out.set_name_and_value(1, "name", str_scalar(&item.name)?)?;
    out.set_name_and_value(2, "kind", str_scalar(&item.kind)?)?;
    out.set_name_and_value(3, "params", string_vector_value(&item.params)?)?;
    out.set_name_and_value(4, "results", string_vector_value(&item.results)?)?;
    out.set_name_and_value(5, "minimum", optional_str_value(item.minimum.as_deref())?)?;
    out.set_name_and_value(6, "maximum", optional_str_value(item.maximum.as_deref())?)?;
    out.set_name_and_value(7, "shared", optional_logical_value(item.shared)?)?;
    out.set_name_and_value(8, "memory64", optional_logical_value(item.memory64)?)?;
    out.set_name_and_value(9, "mutable", optional_logical_value(item.mutable)?)?;
    out.set_name_and_value(10, "element", optional_str_value(item.element.as_deref())?)?;
    out.set_name_and_value(
        11,
        "value_type",
        optional_str_value(item.value_type.as_deref())?,
    )?;
    out.set_name_and_value(12, "signature", str_scalar(&core_item_signature(&item))?)?;
    out.into()
}

fn core_item_signature(item: &CoreItem) -> String {
    match item.kind.as_str() {
        "function" | "tag" => format!(
            "({}) -> ({})",
            item.params.join(", "),
            item.results.join(", ")
        ),
        "memory" => format!(
            "memory min={} max={}{}{}",
            item.minimum.as_deref().unwrap_or("?"),
            item.maximum.as_deref().unwrap_or("unbounded"),
            if item.shared == Some(true) {
                " shared"
            } else {
                ""
            },
            if item.memory64 == Some(true) {
                " memory64"
            } else {
                ""
            }
        ),
        "table" => format!(
            "table {} min={} max={}{}",
            item.element.as_deref().unwrap_or("?"),
            item.minimum.as_deref().unwrap_or("?"),
            item.maximum.as_deref().unwrap_or("unbounded"),
            if item.memory64 == Some(true) {
                " table64"
            } else {
                ""
            }
        ),
        "global" => format!(
            "{} global {}",
            if item.mutable == Some(true) {
                "mutable"
            } else {
                "immutable"
            },
            item.value_type.as_deref().unwrap_or("?")
        ),
        _ => item.kind.clone(),
    }
}

fn component_item_value(item: rwasmtime::component::ComponentItem) -> savvy::Result<Sexp> {
    let mut out = OwnedListSexp::new(5, true)?;
    out.set_name_and_value(0, "name", str_scalar(&item.name)?)?;
    out.set_name_and_value(1, "kind", str_scalar(component_item_kind(item.kind))?)?;
    out.set_name_and_value(
        2,
        "interface",
        optional_str_value(item.interface.as_deref())?,
    )?;
    out.set_name_and_value(
        3,
        "params_schema",
        optional_str_value(item.params_schema.as_deref())?,
    )?;
    out.set_name_and_value(
        4,
        "results_schema",
        optional_str_value(item.results_schema.as_deref())?,
    )?;
    out.into()
}

fn component_item_kind(kind: rwasmtime::component::ComponentItemKind) -> &'static str {
    match kind {
        rwasmtime::component::ComponentItemKind::Function => "function",
        rwasmtime::component::ComponentItemKind::Interface => "interface",
        rwasmtime::component::ComponentItemKind::Resource => "resource",
        rwasmtime::component::ComponentItemKind::World => "world",
    }
}

fn optional_str_value(value: Option<&str>) -> savvy::Result<Sexp> {
    match value {
        Some(value) => str_scalar(value)?.into(),
        None => Ok(NullSexp.into()),
    }
}

fn optional_logical_value(value: Option<bool>) -> savvy::Result<Sexp> {
    match value {
        Some(value) => logical_scalar(value)?.into(),
        None => Ok(NullSexp.into()),
    }
}

fn string_vector_value(values: &[String]) -> savvy::Result<OwnedStringSexp> {
    let mut out = OwnedStringSexp::new(values.len())?;
    for (index, value) in values.iter().enumerate() {
        out.set_elt(index, value)?;
    }
    Ok(out)
}

fn str_scalar(value: &str) -> savvy::Result<OwnedStringSexp> {
    let mut out = OwnedStringSexp::new(1)?;
    out.set_elt(0, value)?;
    Ok(out)
}

fn int_scalar(value: i32) -> savvy::Result<OwnedIntegerSexp> {
    let mut out = OwnedIntegerSexp::new(1)?;
    out.set_elt(0, value)?;
    Ok(out)
}

fn real_scalar(value: f64) -> savvy::Result<OwnedRealSexp> {
    let mut out = OwnedRealSexp::new(1)?;
    out.set_elt(0, value)?;
    Ok(out)
}

fn logical_scalar(value: bool) -> savvy::Result<OwnedLogicalSexp> {
    OwnedLogicalSexp::try_from(value)
}

fn null_ref_value(kind: &str) -> savvy::Result<Sexp> {
    let mut out = OwnedListSexp::new(2, true)?;
    out.set_name_and_value(0, "type", str_scalar(kind)?)?;
    out.set_name_and_value(1, "is_null", logical_scalar(true)?)?;
    out.into()
}

fn to_savvy_error(err: RwasmtimeError) -> savvy::Error {
    savvy::Error::new(&err.message)
}
