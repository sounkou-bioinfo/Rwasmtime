use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};
use std::ptr;

use crate::app::{RwasmtimeError, RwasmtimeErrorKind};
use crate::backend::{CoreExecutionLimits, WasmtimeRuntime};
use crate::config::{CompilerSpec, CompilerStrategy, FeatureSpec, OptLevel, RuntimeSpec};
#[cfg(feature = "wasi")]
use crate::wasi::{StdioMode, WasiSpec};

const STATUS_OK: c_int = 0;
const STATUS_ERR: c_int = 1;
const STATUS_INVALID_ARGUMENT: c_int = 2;
const STATUS_NOT_IMPLEMENTED: c_int = 3;

const TOGGLE_UNSET: u32 = 0;
const TOGGLE_FALSE: u32 = 1;
const TOGGLE_TRUE: u32 = 2;

pub const CORE_VALUE_I32: u32 = 1;
pub const CORE_VALUE_I64: u32 = 2;
pub const CORE_VALUE_F32: u32 = 3;
pub const CORE_VALUE_F64: u32 = 4;

#[cfg(feature = "wasi")]
const STDIO_EMPTY: u32 = 0;
#[cfg(feature = "wasi")]
const STDIO_INHERIT: u32 = 1;
#[cfg(feature = "wasi")]
const STDIO_STRING: u32 = 2;
#[cfg(feature = "wasi")]
const STDIO_FILE: u32 = 3;
#[cfg(feature = "wasi")]
const STDIO_CAPTURE: u32 = 4;
#[cfg(feature = "wasi")]
const STDIO_DISCARD: u32 = 5;

#[repr(C)]
pub struct RuntimeOptions {
    struct_size: usize,
    compiler_strategy: *const c_char,
    opt_level: *const c_char,
    parallel: u32,
    component_model: u32,
    simd: u32,
    relaxed_simd: u32,
    relaxed_simd_deterministic: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CoreValue {
    tag: u32,
    i64_value: i64,
    f64_value: f64,
}

#[repr(C)]
pub struct CoreCallOptions {
    struct_size: usize,
    has_memory_bytes: c_int,
    memory_bytes: u64,
    has_table_elements: c_int,
    table_elements: u64,
    has_instances: c_int,
    instances: u64,
    has_fuel: c_int,
    fuel: u64,
    has_wall_time_ms: c_int,
    wall_time_ms: u64,
}

#[repr(C)]
pub struct WasiPreopenOptions {
    guest: *const c_char,
    host: *const c_char,
    readonly: u32,
}

#[repr(C)]
pub struct WasiOptions {
    args: *const *const c_char,
    args_len: usize,
    env_names: *const *const c_char,
    env_values: *const *const c_char,
    env_len: usize,
    preopens: *const WasiPreopenOptions,
    preopens_len: usize,
    stdin_mode: u32,
    stdout_mode: u32,
    stderr_mode: u32,
    stdin_text: *const c_char,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ByteBuffer {
    data: *mut u8,
    len: usize,
}

pub struct BackendRuntime {
    runtime: WasmtimeRuntime,
}

#[no_mangle]
pub unsafe extern "C" fn rwasmtime_backend_runtime_build(
    opts: *const RuntimeOptions,
    out: *mut *mut BackendRuntime,
    message: *mut *mut c_char,
) -> c_int {
    if !out.is_null() {
        *out = ptr::null_mut();
    }
    if !message.is_null() {
        *message = ptr::null_mut();
    }
    if out.is_null() {
        return set_error(STATUS_INVALID_ARGUMENT, "out must not be NULL", message);
    }

    match runtime_spec_from_options(opts).and_then(WasmtimeRuntime::new) {
        Ok(runtime) => {
            *out = Box::into_raw(Box::new(BackendRuntime { runtime }));
            STATUS_OK
        }
        Err(err) => set_error(status_from_error(&err), &err.message, message),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rwasmtime_backend_runtime_release(runtime: *mut BackendRuntime) {
    if runtime.is_null() {
        return;
    }
    drop(Box::from_raw(runtime));
}

#[no_mangle]
pub unsafe extern "C" fn rwasmtime_backend_call_core(
    runtime: *mut BackendRuntime,
    module_bytes: *const u8,
    module_len: usize,
    export: *const c_char,
    args: *const CoreValue,
    args_len: usize,
    opts: *const CoreCallOptions,
    results: *mut CoreValue,
    results_capacity: usize,
    results_len: *mut usize,
    message: *mut *mut c_char,
) -> c_int {
    if !message.is_null() {
        *message = ptr::null_mut();
    }
    if !results_len.is_null() {
        *results_len = 0;
    }
    if runtime.is_null() {
        return set_error(STATUS_INVALID_ARGUMENT, "runtime must not be NULL", message);
    }
    if module_len > 0 && module_bytes.is_null() {
        return set_error(
            STATUS_INVALID_ARGUMENT,
            "module_bytes must not be NULL when module_len is non-zero",
            message,
        );
    }
    if module_len == 0 {
        return set_error(
            STATUS_INVALID_ARGUMENT,
            "module_len must be non-zero",
            message,
        );
    }
    if args_len > 0 && args.is_null() {
        return set_error(
            STATUS_INVALID_ARGUMENT,
            "args must not be NULL when args_len is non-zero",
            message,
        );
    }
    if results_capacity > 0 && results.is_null() {
        return set_error(
            STATUS_INVALID_ARGUMENT,
            "results must not be NULL when results_capacity is non-zero",
            message,
        );
    }
    if results_len.is_null() {
        return set_error(
            STATUS_INVALID_ARGUMENT,
            "results_len must not be NULL",
            message,
        );
    }

    let module = std::slice::from_raw_parts(module_bytes, module_len);
    let export = match required_str(export, "export") {
        Ok(value) => value,
        Err(err) => return set_error(status_from_error(&err), &err.message, message),
    };
    let args = if args_len == 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(args, args_len)
    };
    let limits = match core_call_limits_from_options(opts) {
        Ok(value) => value,
        Err(err) => return set_error(status_from_error(&err), &err.message, message),
    };
    let runtime = &*runtime;

    match call_core(
        &runtime.runtime,
        module,
        export,
        args,
        results_capacity,
        limits,
    ) {
        Ok(values) => {
            *results_len = values.len();
            for (i, value) in values.iter().enumerate() {
                *results.add(i) = *value;
            }
            STATUS_OK
        }
        Err(err) => set_error(status_from_error(&err), &err.message, message),
    }
}

#[no_mangle]
pub unsafe extern "C" fn rwasmtime_backend_run_wasi_p1_command(
    runtime: *mut BackendRuntime,
    module: *const c_char,
    wasi: *const WasiOptions,
    stdout_out: *mut ByteBuffer,
    stderr_out: *mut ByteBuffer,
    message: *mut *mut c_char,
) -> c_int {
    if !message.is_null() {
        *message = ptr::null_mut();
    }
    clear_buffer(stdout_out);
    clear_buffer(stderr_out);
    if runtime.is_null() {
        return set_error(STATUS_INVALID_ARGUMENT, "runtime must not be NULL", message);
    }
    if stdout_out.is_null() || stderr_out.is_null() {
        return set_error(
            STATUS_INVALID_ARGUMENT,
            "stdout/stderr output buffers must not be NULL",
            message,
        );
    }
    let module = match required_str(module, "module") {
        Ok(value) => value,
        Err(err) => return set_error(status_from_error(&err), &err.message, message),
    };

    #[cfg(feature = "wasi")]
    {
        let wasi = match wasi_spec_from_options(wasi) {
            Ok(value) => value,
            Err(err) => return set_error(status_from_error(&err), &err.message, message),
        };
        let runtime = &*runtime;
        match runtime
            .runtime
            .run_wasi_p1_command(module, &wasi, CoreExecutionLimits::none())
        {
            Ok(output) => {
                *stdout_out = buffer_from_vec(output.stdout);
                *stderr_out = buffer_from_vec(output.stderr);
                STATUS_OK
            }
            Err(err) => set_error(status_from_error(&err), &err.message, message),
        }
    }
    #[cfg(not(feature = "wasi"))]
    {
        let _ = module;
        let _ = wasi;
        set_error(
            STATUS_NOT_IMPLEMENTED,
            "WASI command execution requires a backend built with the wasi feature",
            message,
        )
    }
}

#[no_mangle]
pub unsafe extern "C" fn rwasmtime_backend_bytes_release(data: *mut u8, len: usize) {
    if data.is_null() {
        return;
    }
    drop(Vec::from_raw_parts(data, len, len));
}

#[no_mangle]
pub unsafe extern "C" fn rwasmtime_backend_string_release(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    drop(CString::from_raw(value));
}

fn call_core(
    runtime: &WasmtimeRuntime,
    module: &[u8],
    export: &str,
    args: &[CoreValue],
    results_capacity: usize,
    limits: CoreExecutionLimits,
) -> Result<Vec<CoreValue>, RwasmtimeError> {
    if export.is_empty() {
        return Err(RwasmtimeError::invalid_argument("export must not be empty"));
    }
    let mut instance = runtime.instantiate_core(module, limits)?;
    let ty = instance.func_type(export)?;
    let params: Vec<_> = ty.params().collect();
    let result_tys: Vec<_> = ty.results().collect();
    if params.len() != args.len() {
        return Err(RwasmtimeError::invalid_argument(format!(
            "export `{export}` expects {} argument(s), got {}",
            params.len(),
            args.len()
        )));
    }
    if result_tys.len() > results_capacity {
        return Err(RwasmtimeError::invalid_argument(format!(
            "export `{export}` returns {} value(s), but result capacity is {results_capacity}",
            result_tys.len()
        )));
    }

    let mut wasm_args = Vec::with_capacity(args.len());
    for (index, (value, expected)) in args.iter().zip(params.iter()).enumerate() {
        wasm_args.push(core_value_to_val(*value, expected, index)?);
    }
    let mut wasm_results = result_tys
        .iter()
        .map(default_val_for_type)
        .collect::<Result<Vec<_>, _>>()?;
    instance.call_export(export, &wasm_args, &mut wasm_results)?;

    wasm_results
        .into_iter()
        .enumerate()
        .map(|(index, value)| val_to_core_value(value, index))
        .collect()
}

fn core_value_to_val(
    value: CoreValue,
    expected: &wasmtime::ValType,
    index: usize,
) -> Result<wasmtime::Val, RwasmtimeError> {
    match expected {
        wasmtime::ValType::I32 => Ok(wasmtime::Val::I32(coerce_i32(value, index)?)),
        wasmtime::ValType::I64 => Ok(wasmtime::Val::I64(coerce_i64(value, index)?)),
        wasmtime::ValType::F32 => Ok(wasmtime::Val::F32((coerce_f32(value, index)?).to_bits())),
        wasmtime::ValType::F64 => Ok(wasmtime::Val::F64((coerce_f64(value, index)?).to_bits())),
        other => Err(RwasmtimeError::not_implemented(format!(
            "core value type {other:?} for argument {index}"
        ))),
    }
}

fn default_val_for_type(ty: &wasmtime::ValType) -> Result<wasmtime::Val, RwasmtimeError> {
    match ty {
        wasmtime::ValType::I32 => Ok(wasmtime::Val::I32(0)),
        wasmtime::ValType::I64 => Ok(wasmtime::Val::I64(0)),
        wasmtime::ValType::F32 => Ok(wasmtime::Val::F32(0)),
        wasmtime::ValType::F64 => Ok(wasmtime::Val::F64(0)),
        other => Err(RwasmtimeError::not_implemented(format!(
            "core result value type {other:?}"
        ))),
    }
}

fn val_to_core_value(value: wasmtime::Val, index: usize) -> Result<CoreValue, RwasmtimeError> {
    match value {
        wasmtime::Val::I32(value) => Ok(CoreValue::i32(value)),
        wasmtime::Val::I64(value) => Ok(CoreValue::i64(value)),
        wasmtime::Val::F32(bits) => Ok(CoreValue::f32(f32::from_bits(bits))),
        wasmtime::Val::F64(bits) => Ok(CoreValue::f64(f64::from_bits(bits))),
        other => Err(RwasmtimeError::not_implemented(format!(
            "core result value {other:?} at index {index}"
        ))),
    }
}

impl CoreValue {
    fn i32(value: i32) -> Self {
        Self {
            tag: CORE_VALUE_I32,
            i64_value: value as i64,
            f64_value: value as f64,
        }
    }

    fn i64(value: i64) -> Self {
        Self {
            tag: CORE_VALUE_I64,
            i64_value: value,
            f64_value: value as f64,
        }
    }

    fn f32(value: f32) -> Self {
        Self {
            tag: CORE_VALUE_F32,
            i64_value: value as i64,
            f64_value: value as f64,
        }
    }

    fn f64(value: f64) -> Self {
        Self {
            tag: CORE_VALUE_F64,
            i64_value: value as i64,
            f64_value: value,
        }
    }
}

fn coerce_i32(value: CoreValue, index: usize) -> Result<i32, RwasmtimeError> {
    match value.tag {
        CORE_VALUE_I32 => i32::try_from(value.i64_value).map_err(|_| numeric_error("i32", index)),
        CORE_VALUE_I64 => i32::try_from(value.i64_value).map_err(|_| numeric_error("i32", index)),
        CORE_VALUE_F32 | CORE_VALUE_F64 => {
            if value.f64_value.is_finite()
                && value.f64_value.fract() == 0.0
                && value.f64_value >= i32::MIN as f64
                && value.f64_value <= i32::MAX as f64
            {
                Ok(value.f64_value as i32)
            } else {
                Err(numeric_error("i32", index))
            }
        }
        _ => Err(numeric_error("i32", index)),
    }
}

fn coerce_i64(value: CoreValue, index: usize) -> Result<i64, RwasmtimeError> {
    match value.tag {
        CORE_VALUE_I32 | CORE_VALUE_I64 => Ok(value.i64_value),
        CORE_VALUE_F32 | CORE_VALUE_F64 => {
            if value.f64_value.is_finite()
                && value.f64_value.fract() == 0.0
                && value.f64_value >= i64::MIN as f64
                && value.f64_value <= i64::MAX as f64
            {
                Ok(value.f64_value as i64)
            } else {
                Err(numeric_error("i64", index))
            }
        }
        _ => Err(numeric_error("i64", index)),
    }
}

fn coerce_f32(value: CoreValue, index: usize) -> Result<f32, RwasmtimeError> {
    let value = coerce_f64(value, index)?;
    if value.is_finite() && value >= f32::MIN as f64 && value <= f32::MAX as f64 {
        Ok(value as f32)
    } else {
        Err(numeric_error("f32", index))
    }
}

fn coerce_f64(value: CoreValue, index: usize) -> Result<f64, RwasmtimeError> {
    match value.tag {
        CORE_VALUE_I32 | CORE_VALUE_I64 => Ok(value.i64_value as f64),
        CORE_VALUE_F32 | CORE_VALUE_F64 => Ok(value.f64_value),
        _ => Err(numeric_error("f64", index)),
    }
}

fn numeric_error(expected: &str, index: usize) -> RwasmtimeError {
    RwasmtimeError::invalid_argument(format!(
        "argument {index} cannot be represented as Wasm {expected}"
    ))
}

unsafe fn clear_buffer(out: *mut ByteBuffer) {
    if !out.is_null() {
        (*out).data = ptr::null_mut();
        (*out).len = 0;
    }
}

#[cfg(feature = "wasi")]
fn buffer_from_vec(mut value: Vec<u8>) -> ByteBuffer {
    let out = ByteBuffer {
        data: value.as_mut_ptr(),
        len: value.len(),
    };
    std::mem::forget(value);
    out
}

#[cfg(feature = "wasi")]
unsafe fn wasi_spec_from_options(opts: *const WasiOptions) -> Result<WasiSpec, RwasmtimeError> {
    let Some(opts) = opts.as_ref() else {
        return Ok(WasiSpec::new());
    };
    let mut wasi = WasiSpec::new();

    let args = pointer_slice(opts.args, opts.args_len, "wasi args")?;
    for (index, arg) in args.iter().enumerate() {
        wasi = wasi.arg(required_str(*arg, &format!("wasi arg {index}"))?);
    }

    let env_names = pointer_slice(opts.env_names, opts.env_len, "wasi env names")?;
    let env_values = pointer_slice(opts.env_values, opts.env_len, "wasi env values")?;
    for index in 0..opts.env_len {
        let key = required_str(env_names[index], &format!("wasi env name {index}"))?;
        let value = required_str(env_values[index], &format!("wasi env value {index}"))?;
        wasi = wasi.env(key, value);
    }

    let preopens = pointer_slice(opts.preopens, opts.preopens_len, "wasi preopens")?;
    for (index, preopen) in preopens.iter().enumerate() {
        let guest = required_str(preopen.guest, &format!("wasi preopen guest {index}"))?;
        let host = required_str(preopen.host, &format!("wasi preopen host {index}"))?;
        wasi = wasi.preopen(guest, host, preopen.readonly != 0);
    }

    wasi = wasi.stdio(
        stdio_mode(opts.stdin_mode, "stdin")?,
        stdio_mode(opts.stdout_mode, "stdout")?,
        stdio_mode(opts.stderr_mode, "stderr")?,
    );
    if !opts.stdin_text.is_null() {
        wasi = wasi.stdin_text(required_str(opts.stdin_text, "wasi stdin_text")?);
    }
    Ok(wasi)
}

#[cfg(feature = "wasi")]
unsafe fn pointer_slice<'a, T>(
    ptr: *const T,
    len: usize,
    name: &str,
) -> Result<&'a [T], RwasmtimeError> {
    if len == 0 {
        Ok(&[])
    } else if ptr.is_null() {
        Err(RwasmtimeError::invalid_argument(format!(
            "{name} pointer must not be NULL when length is non-zero"
        )))
    } else {
        Ok(std::slice::from_raw_parts(ptr, len))
    }
}

#[cfg(feature = "wasi")]
fn stdio_mode(value: u32, name: &str) -> Result<StdioMode, RwasmtimeError> {
    match value {
        STDIO_EMPTY => Ok(StdioMode::Empty),
        STDIO_INHERIT => Ok(StdioMode::Inherit),
        STDIO_STRING => Ok(StdioMode::String),
        STDIO_FILE => Ok(StdioMode::File),
        STDIO_CAPTURE => Ok(StdioMode::Capture),
        STDIO_DISCARD => Ok(StdioMode::Discard),
        other => Err(RwasmtimeError::invalid_argument(format!(
            "unsupported WASI {name} stdio mode {other}"
        ))),
    }
}

unsafe fn core_call_limits_from_options(
    opts: *const CoreCallOptions,
) -> Result<CoreExecutionLimits, RwasmtimeError> {
    let Some(opts) = opts.as_ref() else {
        return Ok(CoreExecutionLimits::none());
    };
    if opts.struct_size != std::mem::size_of::<CoreCallOptions>() {
        return Err(RwasmtimeError::invalid_argument(
            "rwasmtime_core_call_options_t has an unsupported struct_size",
        ));
    }
    Ok(CoreExecutionLimits::new(
        option_if_set(opts.has_fuel, opts.fuel),
        option_if_set(opts.has_wall_time_ms, opts.wall_time_ms),
    )
    .resource_limits(
        option_if_set(opts.has_memory_bytes, opts.memory_bytes),
        option_if_set(opts.has_table_elements, opts.table_elements),
        option_if_set(opts.has_instances, opts.instances),
    ))
}

fn option_if_set(has_value: c_int, value: u64) -> Option<u64> {
    if has_value != 0 {
        Some(value)
    } else {
        None
    }
}

unsafe fn runtime_spec_from_options(
    opts: *const RuntimeOptions,
) -> Result<RuntimeSpec, RwasmtimeError> {
    let Some(opts) = opts.as_ref() else {
        return Ok(RuntimeSpec::new());
    };

    let mut compiler = CompilerSpec::auto();
    compiler.strategy = parse_compiler_strategy(opts.compiler_strategy)?;
    compiler.opt_level = parse_opt_level(opts.opt_level)?;
    compiler.parallel = apply_toggle(opts.parallel, compiler.parallel, "parallel")?;

    let mut features = FeatureSpec::new();
    features.component_model = apply_toggle(
        opts.component_model,
        features.component_model,
        "component_model",
    )?;
    features.simd = apply_toggle(opts.simd, features.simd, "simd")?;
    features.relaxed_simd = apply_toggle(opts.relaxed_simd, features.relaxed_simd, "relaxed_simd")?;
    features.relaxed_simd_deterministic = apply_toggle(
        opts.relaxed_simd_deterministic,
        features.relaxed_simd_deterministic,
        "relaxed_simd_deterministic",
    )?;

    Ok(RuntimeSpec::new().compiler(compiler).features(features))
}

unsafe fn parse_compiler_strategy(
    value: *const c_char,
) -> Result<CompilerStrategy, RwasmtimeError> {
    match optional_str(value, "compiler_strategy")? {
        None | Some("auto") => Ok(CompilerStrategy::Auto),
        Some("cranelift") => Ok(CompilerStrategy::Cranelift),
        Some("winch") => Ok(CompilerStrategy::Winch),
        Some(other) => Err(RwasmtimeError::invalid_argument(format!(
            "unsupported compiler_strategy `{other}`"
        ))),
    }
}

unsafe fn parse_opt_level(value: *const c_char) -> Result<OptLevel, RwasmtimeError> {
    match optional_str(value, "opt_level")? {
        None | Some("speed") => Ok(OptLevel::Speed),
        Some("none") => Ok(OptLevel::None),
        Some("speed_and_size") => Ok(OptLevel::SpeedAndSize),
        Some(other) => Err(RwasmtimeError::invalid_argument(format!(
            "unsupported opt_level `{other}`"
        ))),
    }
}

unsafe fn optional_str<'a>(
    value: *const c_char,
    name: &str,
) -> Result<Option<&'a str>, RwasmtimeError> {
    if value.is_null() {
        return Ok(None);
    }
    CStr::from_ptr(value)
        .to_str()
        .map(Some)
        .map_err(|_| RwasmtimeError::invalid_argument(format!("{name} must be UTF-8")))
}

unsafe fn required_str<'a>(value: *const c_char, name: &str) -> Result<&'a str, RwasmtimeError> {
    optional_str(value, name)?
        .ok_or_else(|| RwasmtimeError::invalid_argument(format!("{name} must not be NULL")))
}

fn apply_toggle(value: u32, default: bool, name: &str) -> Result<bool, RwasmtimeError> {
    match value {
        TOGGLE_UNSET => Ok(default),
        TOGGLE_FALSE => Ok(false),
        TOGGLE_TRUE => Ok(true),
        other => Err(RwasmtimeError::invalid_argument(format!(
            "{name} has unsupported toggle value {other}"
        ))),
    }
}

fn status_from_error(err: &RwasmtimeError) -> c_int {
    match err.kind {
        RwasmtimeErrorKind::InvalidArgument => STATUS_INVALID_ARGUMENT,
        RwasmtimeErrorKind::NotImplemented => STATUS_NOT_IMPLEMENTED,
        RwasmtimeErrorKind::Runtime | RwasmtimeErrorKind::AotIncompatible => STATUS_ERR,
    }
}

unsafe fn set_error(status: c_int, message: &str, out: *mut *mut c_char) -> c_int {
    if !out.is_null() {
        let cleaned = message.replace('\0', " ");
        if let Ok(value) = CString::new(cleaned) {
            *out = value.into_raw();
        }
    }
    status
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn build_test_runtime() -> *mut BackendRuntime {
        let compiler = CString::new("cranelift").unwrap();
        let opt = CString::new("speed").unwrap();
        let opts = RuntimeOptions {
            struct_size: std::mem::size_of::<RuntimeOptions>(),
            compiler_strategy: compiler.as_ptr(),
            opt_level: opt.as_ptr(),
            parallel: TOGGLE_TRUE,
            component_model: TOGGLE_FALSE,
            simd: TOGGLE_TRUE,
            relaxed_simd: TOGGLE_FALSE,
            relaxed_simd_deterministic: TOGGLE_FALSE,
        };
        let mut runtime = ptr::null_mut();
        let mut message = ptr::null_mut();
        let status = rwasmtime_backend_runtime_build(&opts, &mut runtime, &mut message);
        assert_eq!(status, STATUS_OK);
        assert!(!runtime.is_null());
        assert!(message.is_null());
        runtime
    }

    #[test]
    fn ffi_builds_real_runtime_with_cranelift_options() {
        unsafe {
            let runtime = build_test_runtime();
            rwasmtime_backend_runtime_release(runtime);
        }
    }

    #[test]
    fn ffi_reports_invalid_feature_combinations() {
        unsafe {
            let opts = RuntimeOptions {
                struct_size: std::mem::size_of::<RuntimeOptions>(),
                compiler_strategy: ptr::null(),
                opt_level: ptr::null(),
                parallel: TOGGLE_UNSET,
                component_model: TOGGLE_UNSET,
                simd: TOGGLE_UNSET,
                relaxed_simd: TOGGLE_FALSE,
                relaxed_simd_deterministic: TOGGLE_TRUE,
            };
            let mut runtime = ptr::null_mut();
            let mut message = ptr::null_mut();
            let status = rwasmtime_backend_runtime_build(&opts, &mut runtime, &mut message);
            assert_eq!(status, STATUS_INVALID_ARGUMENT);
            assert!(runtime.is_null());
            assert!(!message.is_null());
            let text = CStr::from_ptr(message).to_string_lossy().into_owned();
            assert!(text.contains("relaxed_simd_deterministic"));
            rwasmtime_backend_string_release(message);
        }
    }

    #[test]
    fn ffi_generic_call_handles_core_numeric_values() {
        unsafe {
            let runtime = build_test_runtime();
            let module = br#"
                (module
                  (func (export "add") (param i32 i32) (result i32)
                    local.get 0
                    local.get 1
                    i32.add)
                  (func (export "mix") (param i64 f64) (result f64)
                    local.get 0
                    f64.convert_i64_s
                    local.get 1
                    f64.add))
            "#;
            let add = CString::new("add").unwrap();
            let mut add_results = [CoreValue::i32(0); 1];
            let mut add_results_len = 0_usize;
            let add_args = [CoreValue::i32(8), CoreValue::i32(34)];
            let mut message = ptr::null_mut();
            let status = rwasmtime_backend_call_core(
                runtime,
                module.as_ptr(),
                module.len(),
                add.as_ptr(),
                add_args.as_ptr(),
                add_args.len(),
                ptr::null(),
                add_results.as_mut_ptr(),
                add_results.len(),
                &mut add_results_len,
                &mut message,
            );
            assert_eq!(status, STATUS_OK);
            assert_eq!(add_results_len, 1);
            assert_eq!(add_results[0].tag, CORE_VALUE_I32);
            assert_eq!(add_results[0].i64_value, 42);

            let mix = CString::new("mix").unwrap();
            let mut mix_results = [CoreValue::f64(0.0); 1];
            let mut mix_results_len = 0_usize;
            let mix_args = [CoreValue::i64(40), CoreValue::f64(2.5)];
            let opts = CoreCallOptions {
                struct_size: std::mem::size_of::<CoreCallOptions>(),
                has_memory_bytes: 0,
                memory_bytes: 0,
                has_table_elements: 0,
                table_elements: 0,
                has_instances: 0,
                instances: 0,
                has_fuel: 1,
                fuel: 1_000,
                has_wall_time_ms: 0,
                wall_time_ms: 0,
            };
            let status = rwasmtime_backend_call_core(
                runtime,
                module.as_ptr(),
                module.len(),
                mix.as_ptr(),
                mix_args.as_ptr(),
                mix_args.len(),
                &opts,
                mix_results.as_mut_ptr(),
                mix_results.len(),
                &mut mix_results_len,
                &mut message,
            );
            assert_eq!(status, STATUS_OK);
            assert_eq!(mix_results_len, 1);
            assert_eq!(mix_results[0].tag, CORE_VALUE_F64);
            assert_eq!(mix_results[0].f64_value, 42.5);

            rwasmtime_backend_runtime_release(runtime);
        }
    }
}
