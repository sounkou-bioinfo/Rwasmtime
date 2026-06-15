use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::app::{Result, RwasmtimeError, RwasmtimeErrorKind};
use crate::component::{ComponentItem, ComponentItemKind};
use crate::config::{CompilerStrategy, FeatureSpec, OptLevel, RuntimeSpec};
#[cfg(feature = "wasi")]
use crate::wasi::{StdioMode, WasiSpec};

/// Real Wasmtime runtime backend.
///
/// This type is intentionally feature-gated so the default Rust core remains a
/// fast, R-free API scaffold while `make test-rust-backend` exercises the real
/// runtime path.
#[derive(Debug, Clone)]
pub struct WasmtimeRuntime {
    spec: RuntimeSpec,
    engine: wasmtime::Engine,
}

#[cfg(feature = "wasi")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasiCommandOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug)]
struct CoreStoreState {
    resource_limits: RuntimeStoreLimits,
}

impl CoreStoreState {
    fn new(limits: CoreExecutionLimits) -> Result<Self> {
        Ok(Self {
            resource_limits: RuntimeStoreLimits::new(limits)?,
        })
    }
}

#[cfg(feature = "wasi")]
struct WasiP1State {
    wasi: wasmtime_wasi::p1::WasiP1Ctx,
    resource_limits: RuntimeStoreLimits,
}

#[derive(Debug, Clone)]
pub struct CoreFuncImport {
    pub module: String,
    pub name: String,
    pub params: Vec<wasmtime::ValType>,
    pub results: Vec<wasmtime::ValType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoreItem {
    pub module: Option<String>,
    pub name: String,
    pub kind: String,
    pub params: Vec<String>,
    pub results: Vec<String>,
    pub minimum: Option<String>,
    pub maximum: Option<String>,
    pub shared: Option<bool>,
    pub memory64: Option<bool>,
    pub mutable: Option<bool>,
    pub element: Option<String>,
    pub value_type: Option<String>,
}

pub type CoreHostFuncCallback = dyn Fn(&str, &[wasmtime::Val], &mut [wasmtime::Val]) -> wasmtime::Result<()>
    + Send
    + Sync
    + 'static;
pub type CoreHostMemoryRequestCallback =
    dyn Fn(&str, &[u8], &[u8], u32) -> wasmtime::Result<Vec<u8>> + Send + Sync + 'static;

#[derive(Clone)]
enum CoreHostFuncCallbackKind {
    Core(Arc<CoreHostFuncCallback>),
    MemoryRequest(Arc<CoreHostMemoryRequestCallback>),
}

const UNLIMITED_FUEL_SENTINEL: u64 = u64::MAX / 4;
const UNLIMITED_EPOCH_DEADLINE: u64 = u64::MAX / 4;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoreExecutionLimits {
    pub memory_bytes: Option<u64>,
    pub table_elements: Option<u64>,
    pub instances: Option<u64>,
    pub fuel: Option<u64>,
    pub wall_time_ms: Option<u64>,
}

impl CoreExecutionLimits {
    pub fn none() -> Self {
        Self {
            memory_bytes: None,
            table_elements: None,
            instances: None,
            fuel: None,
            wall_time_ms: None,
        }
    }

    pub fn new(fuel: Option<u64>, wall_time_ms: Option<u64>) -> Self {
        Self {
            fuel,
            wall_time_ms,
            ..Self::none()
        }
    }

    pub fn resource_limits(
        mut self,
        memory_bytes: Option<u64>,
        table_elements: Option<u64>,
        instances: Option<u64>,
    ) -> Self {
        self.memory_bytes = memory_bytes;
        self.table_elements = table_elements;
        self.instances = instances;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeStoreLimits {
    memory_bytes: Option<usize>,
    table_elements: Option<usize>,
    instances: usize,
}

impl RuntimeStoreLimits {
    fn new(limits: CoreExecutionLimits) -> Result<Self> {
        Ok(Self {
            memory_bytes: optional_limit_usize(limits.memory_bytes, "memory")?,
            table_elements: optional_limit_usize(limits.table_elements, "table element")?,
            instances: optional_limit_usize(limits.instances, "instance")?
                .unwrap_or(wasmtime::DEFAULT_INSTANCE_LIMIT),
        })
    }
}

impl wasmtime::ResourceLimiter for RuntimeStoreLimits {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if let Some(limit) = self.memory_bytes {
            if desired > limit {
                return Err(wasmtime::format_err!(
                    "memory limit exceeded: requested {desired} bytes exceeds configured memory limit {limit} bytes"
                ));
            }
        }
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if let Some(limit) = self.table_elements {
            if desired > limit {
                return Err(wasmtime::format_err!(
                    "table element limit exceeded: requested {desired} elements exceeds configured table element limit {limit} elements"
                ));
            }
        }
        Ok(true)
    }

    fn instances(&self) -> usize {
        self.instances
    }
}

fn optional_limit_usize(value: Option<u64>, label: &str) -> Result<Option<usize>> {
    value
        .map(|value| {
            usize::try_from(value).map_err(|_| {
                RwasmtimeError::invalid_argument(format!(
                    "{label} limit exceeds this platform's addressable size"
                ))
            })
        })
        .transpose()
}

#[derive(Clone)]
pub struct CoreHostFunc {
    pub module: String,
    pub name: String,
    pub params: Vec<wasmtime::ValType>,
    pub results: Vec<wasmtime::ValType>,
    callback: CoreHostFuncCallbackKind,
}

impl fmt::Debug for CoreHostFunc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CoreHostFunc")
            .field("module", &self.module)
            .field("name", &self.name)
            .field("params", &self.params)
            .field("results", &self.results)
            .finish_non_exhaustive()
    }
}

impl CoreHostFunc {
    pub fn new(
        module: impl Into<String>,
        name: impl Into<String>,
        params: Vec<wasmtime::ValType>,
        results: Vec<wasmtime::ValType>,
        callback: Arc<CoreHostFuncCallback>,
    ) -> Self {
        Self {
            module: module.into(),
            name: name.into(),
            params,
            results,
            callback: CoreHostFuncCallbackKind::Core(callback),
        }
    }

    pub fn new_memory_request(
        module: impl Into<String>,
        name: impl Into<String>,
        callback: Arc<CoreHostMemoryRequestCallback>,
    ) -> Self {
        Self {
            module: module.into(),
            name: name.into(),
            params: vec![
                wasmtime::ValType::I32,
                wasmtime::ValType::I32,
                wasmtime::ValType::I32,
                wasmtime::ValType::I32,
                wasmtime::ValType::I32,
                wasmtime::ValType::I32,
            ],
            results: vec![wasmtime::ValType::I32],
            callback: CoreHostFuncCallbackKind::MemoryRequest(callback),
        }
    }

    fn key(&self) -> String {
        format!("{}::{}", self.module, self.name)
    }
}

fn define_core_host_funcs<T: 'static>(
    engine: &wasmtime::Engine,
    linker: &mut wasmtime::Linker<T>,
    host_funcs: Vec<CoreHostFunc>,
) -> Result<()> {
    for host in host_funcs {
        let key = host.key();
        let key_for_callback = key.clone();
        let ty = wasmtime::FuncType::new(engine, host.params.clone(), host.results.clone());
        let callback = host.callback.clone();
        linker
            .func_new(
                &host.module,
                &host.name,
                ty,
                move |mut caller, args, results| match &callback {
                    CoreHostFuncCallbackKind::Core(callback) => {
                        callback(&key_for_callback, args, results)
                    }
                    CoreHostFuncCallbackKind::MemoryRequest(callback) => {
                        invoke_core_memory_request(
                            &mut caller,
                            &key_for_callback,
                            callback,
                            args,
                            results,
                        )
                    }
                },
            )
            .map_err(|err| {
                RwasmtimeError::runtime(format!("failed to link callback import `{key}`: {err}"))
            })?;
    }
    Ok(())
}

fn invoke_core_memory_request<T: 'static>(
    caller: &mut wasmtime::Caller<'_, T>,
    key: &str,
    callback: &Arc<CoreHostMemoryRequestCallback>,
    args: &[wasmtime::Val],
    results: &mut [wasmtime::Val],
) -> wasmtime::Result<()> {
    let [wasmtime::Val::I32(name_ptr), wasmtime::Val::I32(name_len), wasmtime::Val::I32(payload_ptr), wasmtime::Val::I32(payload_len), wasmtime::Val::I32(result_ptr), wasmtime::Val::I32(result_cap)] =
        args
    else {
        return Err(wasmtime::format_err!(
            "core memory request callback `{key}` received an unexpected argument signature"
        ));
    };
    if results.len() != 1 {
        return Err(wasmtime::format_err!(
            "core memory request callback `{key}` received an unexpected result signature"
        ));
    }
    let Some(memory) = caller
        .get_export("memory")
        .and_then(|item| item.into_memory())
    else {
        results[0] = wasmtime::Val::I32(-1);
        return Ok(());
    };
    let name_ptr = nonnegative_i32_to_usize(*name_ptr, "name_ptr", key)?;
    let name_len = nonnegative_i32_to_usize(*name_len, "name_len", key)?;
    let payload_ptr = nonnegative_i32_to_usize(*payload_ptr, "payload_ptr", key)?;
    let payload_len = nonnegative_i32_to_usize(*payload_len, "payload_len", key)?;
    let result_ptr = nonnegative_i32_to_usize(*result_ptr, "result_ptr", key)?;
    let result_cap_i32 = *result_cap;
    if result_cap_i32 < 0 {
        return Err(wasmtime::format_err!(
            "core memory request callback `{key}` received negative result_cap"
        ));
    }
    let result_cap = result_cap_i32 as u32;

    let mut name = vec![0; name_len];
    memory.read(&*caller, name_ptr, &mut name).map_err(|_| {
        wasmtime::format_err!("core memory request callback `{key}` could not read name bytes")
    })?;
    let mut payload = vec![0; payload_len];
    memory
        .read(&*caller, payload_ptr, &mut payload)
        .map_err(|_| {
            wasmtime::format_err!(
                "core memory request callback `{key}` could not read payload bytes"
            )
        })?;

    let response = callback(key, &name, &payload, result_cap)?;
    let response_len = i32::try_from(response.len()).map_err(|_| {
        wasmtime::format_err!(
            "core memory request callback `{key}` returned more than i32::MAX bytes"
        )
    })?;
    if response.len() <= result_cap as usize {
        memory.write(caller, result_ptr, &response).map_err(|_| {
            wasmtime::format_err!(
                "core memory request callback `{key}` could not write result bytes"
            )
        })?;
    }
    results[0] = wasmtime::Val::I32(response_len);
    Ok(())
}

fn nonnegative_i32_to_usize(value: i32, label: &str, key: &str) -> wasmtime::Result<usize> {
    if value < 0 {
        return Err(wasmtime::format_err!(
            "core memory request callback `{key}` received negative {label}"
        ));
    }
    Ok(value as usize)
}

fn describe_core_item(module: Option<&str>, name: &str, ty: wasmtime::ExternType) -> CoreItem {
    let mut item = CoreItem {
        module: module.map(str::to_string),
        name: name.to_string(),
        kind: String::new(),
        params: Vec::new(),
        results: Vec::new(),
        minimum: None,
        maximum: None,
        shared: None,
        memory64: None,
        mutable: None,
        element: None,
        value_type: None,
    };
    match ty {
        wasmtime::ExternType::Func(func) => {
            item.kind = "function".to_string();
            item.params = func.params().map(|ty| ty.to_string()).collect();
            item.results = func.results().map(|ty| ty.to_string()).collect();
        }
        wasmtime::ExternType::Memory(memory) => {
            item.kind = "memory".to_string();
            item.minimum = Some(memory.minimum().to_string());
            item.maximum = memory.maximum().map(|value| value.to_string());
            item.shared = Some(memory.is_shared());
            item.memory64 = Some(memory.is_64());
        }
        wasmtime::ExternType::Table(table) => {
            item.kind = "table".to_string();
            item.minimum = Some(table.minimum().to_string());
            item.maximum = table.maximum().map(|value| value.to_string());
            item.memory64 = Some(table.is_64());
            item.element = Some(table.element().to_string());
        }
        wasmtime::ExternType::Global(global) => {
            item.kind = "global".to_string();
            item.value_type = Some(global.content().to_string());
            item.mutable = Some(global.mutability().is_var());
        }
        wasmtime::ExternType::Tag(tag) => {
            item.kind = "tag".to_string();
            item.params = tag.ty().params().map(|ty| ty.to_string()).collect();
            item.results = tag.ty().results().map(|ty| ty.to_string()).collect();
        }
    }
    item
}

#[derive(Debug, Clone)]
pub struct CoreModule {
    module: wasmtime::Module,
}

impl CoreModule {
    pub fn func_imports(&self) -> Vec<CoreFuncImport> {
        self.module
            .imports()
            .filter_map(|import| match import.ty() {
                wasmtime::ExternType::Func(ty) => Some(CoreFuncImport {
                    module: import.module().to_string(),
                    name: import.name().to_string(),
                    params: ty.params().collect(),
                    results: ty.results().collect(),
                }),
                _ => None,
            })
            .collect()
    }

    pub fn imports(&self) -> Vec<CoreItem> {
        self.module
            .imports()
            .map(|import| describe_core_item(Some(import.module()), import.name(), import.ty()))
            .collect()
    }

    pub fn exports(&self) -> Vec<CoreItem> {
        self.module
            .exports()
            .map(|export| describe_core_item(None, export.name(), export.ty()))
            .collect()
    }

    pub fn instantiate(&self, limits: CoreExecutionLimits) -> Result<CoreInstance> {
        let mut store = new_core_store(self.module.engine(), limits)?;
        let wall_clock = configure_store_limits(&mut store, limits)?;
        let _wall_time_guard = enter_wall_time_call(&mut store, &wall_clock);
        let instance = wasmtime::Instance::new(&mut store, &self.module, &[]).map_err(|err| {
            RwasmtimeError::runtime(format!("failed to instantiate Wasm module: {err:#}"))
        })?;
        Ok(CoreInstance::new_empty(store, instance, limits, wall_clock))
    }

    pub fn instantiate_with_host_funcs(
        &self,
        host_funcs: Vec<CoreHostFunc>,
        limits: CoreExecutionLimits,
    ) -> Result<CoreInstance> {
        let mut linker = wasmtime::Linker::new(self.module.engine());
        define_core_host_funcs(self.module.engine(), &mut linker, host_funcs)?;
        let mut store = new_core_store(self.module.engine(), limits)?;
        let wall_clock = configure_store_limits(&mut store, limits)?;
        let _wall_time_guard = enter_wall_time_call(&mut store, &wall_clock);
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|err| {
                RwasmtimeError::runtime(format!(
                    "failed to instantiate Wasm module with callbacks: {err:#}"
                ))
            })?;
        Ok(CoreInstance::new_empty(store, instance, limits, wall_clock))
    }

    #[cfg(feature = "wasi")]
    pub fn instantiate_wasi_p1(
        &self,
        wasi: &WasiSpec,
        limits: CoreExecutionLimits,
    ) -> Result<CoreInstance> {
        self.instantiate_wasi_p1_with_host_funcs(wasi, Vec::new(), limits)
    }

    #[cfg(feature = "wasi")]
    pub fn instantiate_wasi_p1_with_host_funcs(
        &self,
        wasi: &WasiSpec,
        host_funcs: Vec<CoreHostFunc>,
        limits: CoreExecutionLimits,
    ) -> Result<CoreInstance> {
        wasi.validate()?;
        validate_supported_wasi_backend(wasi)?;

        let mut linker = wasmtime::Linker::new(self.module.engine());
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut WasiP1State| {
            &mut state.wasi
        })
        .map_err(|err| RwasmtimeError::runtime(format!("failed to link WASIp1 imports: {err}")))?;
        define_core_host_funcs(self.module.engine(), &mut linker, host_funcs)?;

        let (state, stdout_capture, stderr_capture) = build_wasi_p1_state(wasi, limits)?;
        let mut store = new_wasi_p1_store(self.module.engine(), state)?;
        let wall_clock = configure_store_limits(&mut store, limits)?;
        let _wall_time_guard = enter_wall_time_call(&mut store, &wall_clock);
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|err| {
                RwasmtimeError::runtime(format!("failed to instantiate WASIp1 module: {err:#}"))
            })?;
        Ok(CoreInstance::new_wasi_p1(
            store,
            instance,
            stdout_capture,
            stderr_capture,
            limits,
            wall_clock,
        ))
    }

    pub fn serialize(&self) -> Result<Vec<u8>> {
        self.module.serialize().map_err(|err| {
            RwasmtimeError::runtime(format!("failed to serialize compiled core module: {err}"))
        })
    }
}

enum CoreStore {
    Empty(wasmtime::Store<CoreStoreState>),
    #[cfg(feature = "wasi")]
    WasiP1(wasmtime::Store<WasiP1State>),
}

pub struct CoreInstance {
    store: CoreStore,
    instance: wasmtime::Instance,
    limits: CoreExecutionLimits,
    wall_clock: Option<Arc<Mutex<Option<Instant>>>>,
    #[cfg(feature = "wasi")]
    stdout_capture: Option<wasmtime_wasi::p2::pipe::MemoryOutputPipe>,
    #[cfg(feature = "wasi")]
    stderr_capture: Option<wasmtime_wasi::p2::pipe::MemoryOutputPipe>,
}

impl CoreInstance {
    fn new_empty(
        store: wasmtime::Store<CoreStoreState>,
        instance: wasmtime::Instance,
        limits: CoreExecutionLimits,
        wall_clock: Option<Arc<Mutex<Option<Instant>>>>,
    ) -> Self {
        Self {
            store: CoreStore::Empty(store),
            instance,
            limits,
            wall_clock,
            #[cfg(feature = "wasi")]
            stdout_capture: None,
            #[cfg(feature = "wasi")]
            stderr_capture: None,
        }
    }

    #[cfg(feature = "wasi")]
    fn new_wasi_p1(
        store: wasmtime::Store<WasiP1State>,
        instance: wasmtime::Instance,
        stdout_capture: Option<wasmtime_wasi::p2::pipe::MemoryOutputPipe>,
        stderr_capture: Option<wasmtime_wasi::p2::pipe::MemoryOutputPipe>,
        limits: CoreExecutionLimits,
        wall_clock: Option<Arc<Mutex<Option<Instant>>>>,
    ) -> Self {
        Self {
            store: CoreStore::WasiP1(store),
            instance,
            limits,
            wall_clock,
            stdout_capture,
            stderr_capture,
        }
    }

    pub fn func_type(&mut self, export: &str) -> Result<wasmtime::FuncType> {
        if export.is_empty() {
            return Err(RwasmtimeError::invalid_argument("export must not be empty"));
        }
        match &mut self.store {
            CoreStore::Empty(store) => func_type_in_store(&self.instance, store, export),
            #[cfg(feature = "wasi")]
            CoreStore::WasiP1(store) => func_type_in_store(&self.instance, store, export),
        }
    }

    pub fn call_export(
        &mut self,
        export: &str,
        args: &[wasmtime::Val],
        results: &mut [wasmtime::Val],
    ) -> Result<()> {
        if export.is_empty() {
            return Err(RwasmtimeError::invalid_argument("export must not be empty"));
        }
        match &mut self.store {
            CoreStore::Empty(store) => call_export_with_limits(
                &self.instance,
                store,
                self.limits,
                &self.wall_clock,
                export,
                args,
                results,
            ),
            #[cfg(feature = "wasi")]
            CoreStore::WasiP1(store) => call_export_with_limits(
                &self.instance,
                store,
                self.limits,
                &self.wall_clock,
                export,
                args,
                results,
            ),
        }
    }

    pub fn memory_size_pages(&mut self, name: &str) -> Result<u64> {
        match &mut self.store {
            CoreStore::Empty(store) => memory_size_in_store(&self.instance, store, name),
            #[cfg(feature = "wasi")]
            CoreStore::WasiP1(store) => memory_size_in_store(&self.instance, store, name),
        }
    }

    pub fn memory_grow_pages(&mut self, name: &str, pages: u64) -> Result<u64> {
        match &mut self.store {
            CoreStore::Empty(store) => memory_grow_in_store(&self.instance, store, name, pages),
            #[cfg(feature = "wasi")]
            CoreStore::WasiP1(store) => memory_grow_in_store(&self.instance, store, name, pages),
        }
    }

    pub fn memory_read(&mut self, name: &str, offset: usize, len: usize) -> Result<Vec<u8>> {
        match &mut self.store {
            CoreStore::Empty(store) => {
                memory_read_in_store(&self.instance, store, name, offset, len)
            }
            #[cfg(feature = "wasi")]
            CoreStore::WasiP1(store) => {
                memory_read_in_store(&self.instance, store, name, offset, len)
            }
        }
    }

    pub fn memory_write(&mut self, name: &str, offset: usize, bytes: &[u8]) -> Result<()> {
        match &mut self.store {
            CoreStore::Empty(store) => {
                memory_write_in_store(&self.instance, store, name, offset, bytes)
            }
            #[cfg(feature = "wasi")]
            CoreStore::WasiP1(store) => {
                memory_write_in_store(&self.instance, store, name, offset, bytes)
            }
        }
    }

    #[cfg(feature = "wasi")]
    pub fn wasi_output(&self) -> WasiCommandOutput {
        WasiCommandOutput {
            stdout: self
                .stdout_capture
                .as_ref()
                .map(|pipe| pipe.contents().to_vec())
                .unwrap_or_default(),
            stderr: self
                .stderr_capture
                .as_ref()
                .map(|pipe| pipe.contents().to_vec())
                .unwrap_or_default(),
        }
    }
}

fn new_core_store(
    engine: &wasmtime::Engine,
    limits: CoreExecutionLimits,
) -> Result<wasmtime::Store<CoreStoreState>> {
    let mut store = wasmtime::Store::new(engine, CoreStoreState::new(limits)?);
    store.limiter(|state| &mut state.resource_limits);
    Ok(store)
}

#[cfg(feature = "wasi")]
fn new_wasi_p1_store(
    engine: &wasmtime::Engine,
    state: WasiP1State,
) -> Result<wasmtime::Store<WasiP1State>> {
    let mut store = wasmtime::Store::new(engine, state);
    store.limiter(|state| &mut state.resource_limits);
    Ok(store)
}

fn func_type_in_store<T>(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<T>,
    export: &str,
) -> Result<wasmtime::FuncType> {
    let func = instance
        .get_func(&mut *store, export)
        .ok_or_else(|| RwasmtimeError::runtime(format!("failed to resolve export `{export}`")))?;
    Ok(func.ty(&*store))
}

fn configure_store_limits<T>(
    store: &mut wasmtime::Store<T>,
    limits: CoreExecutionLimits,
) -> Result<Option<Arc<Mutex<Option<Instant>>>>> {
    let fuel = limits.fuel.unwrap_or(UNLIMITED_FUEL_SENTINEL);
    store.set_fuel(fuel).map_err(|err| {
        RwasmtimeError::runtime(format!("failed to configure Wasm fuel limit: {err}"))
    })?;

    #[cfg(target_has_atomic = "64")]
    {
        if let Some(ms) = limits.wall_time_ms {
            let wall_clock = Arc::new(Mutex::new(None::<Instant>));
            let wall_clock_for_callback = Arc::clone(&wall_clock);
            let limit = Duration::from_millis(ms);
            store.epoch_deadline_callback(move |_| {
                let elapsed = match wall_clock_for_callback.lock() {
                    Ok(guard) => guard.as_ref().map(|start| start.elapsed()),
                    Err(_) => None,
                };
                if elapsed.is_some_and(|elapsed| elapsed >= limit) {
                    return Err(wasmtime::format_err!(
                        "wall time limit exceeded after {ms} ms"
                    ));
                }
                Ok(wasmtime::UpdateDeadline::Continue(1))
            });
            store.set_epoch_deadline(UNLIMITED_EPOCH_DEADLINE);
            return Ok(Some(wall_clock));
        }
        store.set_epoch_deadline(UNLIMITED_EPOCH_DEADLINE);
    }

    #[cfg(not(target_has_atomic = "64"))]
    if limits.wall_time_ms.is_some() {
        return Err(RwasmtimeError::not_implemented(
            "wall time limits require a target with 64-bit atomics for Wasmtime epoch interruption",
        ));
    }

    Ok(None)
}

struct WallTimeCallGuard {
    wall_clock: Option<Arc<Mutex<Option<Instant>>>>,
    stop: Option<Arc<AtomicBool>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl Drop for WallTimeCallGuard {
    fn drop(&mut self) {
        if let Some(stop) = &self.stop {
            stop.store(true, Ordering::Relaxed);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
        if let Some(wall_clock) = &self.wall_clock {
            if let Ok(mut guard) = wall_clock.lock() {
                *guard = None;
            }
        }
    }
}

fn enter_wall_time_call<T>(
    store: &mut wasmtime::Store<T>,
    wall_clock: &Option<Arc<Mutex<Option<Instant>>>>,
) -> WallTimeCallGuard {
    let Some(wall_clock) = wall_clock else {
        return WallTimeCallGuard {
            wall_clock: None,
            stop: None,
            handle: None,
        };
    };

    #[cfg(target_has_atomic = "64")]
    {
        if let Ok(mut guard) = wall_clock.lock() {
            *guard = Some(Instant::now());
        }
        store.set_epoch_deadline(0);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = Arc::clone(&stop);
        let engine = store.engine().clone();
        let handle = thread::spawn(move || {
            while !stop_for_thread.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(1));
                engine.increment_epoch();
            }
        });
        WallTimeCallGuard {
            wall_clock: Some(Arc::clone(wall_clock)),
            stop: Some(stop),
            handle: Some(handle),
        }
    }

    #[cfg(not(target_has_atomic = "64"))]
    {
        let _ = store;
        WallTimeCallGuard {
            wall_clock: None,
            stop: None,
            handle: None,
        }
    }
}

fn call_export_with_limits<T>(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<T>,
    _limits: CoreExecutionLimits,
    wall_clock: &Option<Arc<Mutex<Option<Instant>>>>,
    export: &str,
    args: &[wasmtime::Val],
    results: &mut [wasmtime::Val],
) -> Result<()> {
    let _wall_time_guard = enter_wall_time_call(store, wall_clock);
    call_export_in_store(instance, store, export, args, results)
}

fn call_export_in_store<T>(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<T>,
    export: &str,
    args: &[wasmtime::Val],
    results: &mut [wasmtime::Val],
) -> Result<()> {
    let func = instance
        .get_func(&mut *store, export)
        .ok_or_else(|| RwasmtimeError::runtime(format!("failed to resolve export `{export}`")))?;
    func.call(&mut *store, args, results).map_err(|err| {
        RwasmtimeError::runtime(format!("Wasm call `{export}` trapped or failed: {err:#}"))
    })
}

fn memory_in_store<T>(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<T>,
    name: &str,
) -> Result<wasmtime::Memory> {
    if name.is_empty() {
        return Err(RwasmtimeError::invalid_argument(
            "memory name must not be empty",
        ));
    }
    instance
        .get_memory(&mut *store, name)
        .ok_or_else(|| RwasmtimeError::runtime(format!("failed to resolve memory export `{name}`")))
}

fn memory_size_in_store<T>(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<T>,
    name: &str,
) -> Result<u64> {
    let memory = memory_in_store(instance, store, name)?;
    Ok(memory.size(&*store))
}

fn memory_grow_in_store<T>(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<T>,
    name: &str,
    pages: u64,
) -> Result<u64> {
    let memory = memory_in_store(instance, store, name)?;
    memory
        .grow(&mut *store, pages)
        .map_err(|err| RwasmtimeError::runtime(format!("failed to grow memory `{name}`: {err}")))
}

fn memory_read_in_store<T>(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<T>,
    name: &str,
    offset: usize,
    len: usize,
) -> Result<Vec<u8>> {
    let memory = memory_in_store(instance, store, name)?;
    let data = memory.data(&*store);
    let end = offset
        .checked_add(len)
        .ok_or_else(|| RwasmtimeError::invalid_argument("memory read range overflows usize"))?;
    if end > data.len() {
        return Err(RwasmtimeError::invalid_argument(format!(
            "memory read range [{}..{}) exceeds memory `{name}` length {}",
            offset,
            end,
            data.len()
        )));
    }
    Ok(data[offset..end].to_vec())
}

fn memory_write_in_store<T>(
    instance: &wasmtime::Instance,
    store: &mut wasmtime::Store<T>,
    name: &str,
    offset: usize,
    bytes: &[u8],
) -> Result<()> {
    let memory = memory_in_store(instance, store, name)?;
    let data = memory.data_mut(&mut *store);
    let end = offset
        .checked_add(bytes.len())
        .ok_or_else(|| RwasmtimeError::invalid_argument("memory write range overflows usize"))?;
    if end > data.len() {
        return Err(RwasmtimeError::invalid_argument(format!(
            "memory write range [{}..{}) exceeds memory `{name}` length {}",
            offset,
            end,
            data.len()
        )));
    }
    data[offset..end].copy_from_slice(bytes);
    Ok(())
}

impl WasmtimeRuntime {
    pub fn new(spec: RuntimeSpec) -> Result<Self> {
        let config = build_config(&spec)?;
        let engine = wasmtime::Engine::new(&config).map_err(|err| {
            RwasmtimeError::runtime(format!("failed to build Wasmtime engine: {err}"))
        })?;
        Ok(Self { spec, engine })
    }

    pub fn spec(&self) -> &RuntimeSpec {
        &self.spec
    }

    pub fn engine(&self) -> &wasmtime::Engine {
        &self.engine
    }

    pub fn compile_core(&self, module: impl AsRef<[u8]>) -> Result<CoreModule> {
        let module = wasmtime::Module::new(&self.engine, module).map_err(|err| {
            RwasmtimeError::runtime(format!("failed to compile Wasm module: {err:#}"))
        })?;
        Ok(CoreModule { module })
    }

    pub fn deserialize_core(&self, bytes: impl AsRef<[u8]>) -> Result<CoreModule> {
        // SAFETY: callers must only pass artifacts that were produced by this
        // package/Wasmtime compatibility line and validated by metadata before
        // deserialization. The R adapter enforces this for public AOT loading.
        let module =
            unsafe { wasmtime::Module::deserialize(&self.engine, bytes) }.map_err(|err| {
                RwasmtimeError::runtime(format!(
                    "failed to deserialize compiled core module: {err}"
                ))
            })?;
        Ok(CoreModule { module })
    }

    pub fn component_imports(&self, component: impl AsRef<[u8]>) -> Result<Vec<ComponentItem>> {
        let component = self.compile_component_for_introspection(component)?;
        Ok(component
            .component_type()
            .imports(&self.engine)
            .map(|(name, item)| describe_component_item(name, item))
            .collect())
    }

    pub fn component_exports(&self, component: impl AsRef<[u8]>) -> Result<Vec<ComponentItem>> {
        let component = self.compile_component_for_introspection(component)?;
        Ok(component
            .component_type()
            .exports(&self.engine)
            .map(|(name, item)| describe_component_item(name, item))
            .collect())
    }

    fn compile_component_for_introspection(
        &self,
        component: impl AsRef<[u8]>,
    ) -> Result<wasmtime::component::Component> {
        wasmtime::component::Component::new(&self.engine, component).map_err(|err| {
            RwasmtimeError::runtime(format!(
                "failed to compile Wasm component for introspection: {err:#}"
            ))
        })
    }

    pub fn instantiate_core_module(
        &self,
        module: &CoreModule,
        limits: CoreExecutionLimits,
    ) -> Result<CoreInstance> {
        self.ensure_module_engine(module)?;
        module.instantiate(limits)
    }

    pub fn instantiate_core_module_with_host_funcs(
        &self,
        module: &CoreModule,
        host_funcs: Vec<CoreHostFunc>,
        limits: CoreExecutionLimits,
    ) -> Result<CoreInstance> {
        self.ensure_module_engine(module)?;
        module.instantiate_with_host_funcs(host_funcs, limits)
    }

    #[cfg(feature = "wasi")]
    pub fn instantiate_core_module_wasi_p1(
        &self,
        module: &CoreModule,
        wasi: &WasiSpec,
        limits: CoreExecutionLimits,
    ) -> Result<CoreInstance> {
        self.ensure_module_engine(module)?;
        module.instantiate_wasi_p1(wasi, limits)
    }

    #[cfg(feature = "wasi")]
    pub fn instantiate_core_module_wasi_p1_with_host_funcs(
        &self,
        module: &CoreModule,
        wasi: &WasiSpec,
        host_funcs: Vec<CoreHostFunc>,
        limits: CoreExecutionLimits,
    ) -> Result<CoreInstance> {
        self.ensure_module_engine(module)?;
        module.instantiate_wasi_p1_with_host_funcs(wasi, host_funcs, limits)
    }

    fn ensure_module_engine(&self, module: &CoreModule) -> Result<()> {
        if !wasmtime::Engine::same(&self.engine, module.module.engine()) {
            return Err(RwasmtimeError::invalid_argument(
                "compiled core module belongs to a different Wasmtime engine",
            ));
        }
        Ok(())
    }

    pub fn instantiate_core(
        &self,
        module: impl AsRef<[u8]>,
        limits: CoreExecutionLimits,
    ) -> Result<CoreInstance> {
        self.compile_core(module)?.instantiate(limits)
    }

    /// Compile and instantiate a core module, then call an export using
    /// Wasmtime's dynamic core value ABI.
    ///
    /// This is intentionally generic: higher adapter layers decide how R or C
    /// values are copied into `wasmtime::Val` values and how results are copied
    /// back out. Do not add one backend entry point per Wasm signature.
    pub fn call_core_export(
        &self,
        module: impl AsRef<[u8]>,
        export: &str,
        args: &[wasmtime::Val],
        results: &mut [wasmtime::Val],
        limits: CoreExecutionLimits,
    ) -> Result<()> {
        if export.is_empty() {
            return Err(RwasmtimeError::invalid_argument("export must not be empty"));
        }

        let mut instance = self.instantiate_core(module, limits)?;
        instance.call_export(export, args, results)
    }

    /// Compile and run a WASIp1 command module using the package `WasiSpec`.
    ///
    /// This is the first real WASI backend boundary: no ambient filesystem,
    /// environment, or inherited stdio is granted unless it is visible in the
    /// supplied spec. It remains intentionally narrow and returns copied stdout
    /// and stderr buffers instead of exposing host/R objects.
    #[cfg(feature = "wasi")]
    pub fn run_wasi_p1_command(
        &self,
        module: impl AsRef<[u8]>,
        wasi: &WasiSpec,
        limits: CoreExecutionLimits,
    ) -> Result<WasiCommandOutput> {
        wasi.validate()?;
        validate_supported_wasi_backend(wasi)?;

        let module = wasmtime::Module::new(&self.engine, module).map_err(|err| {
            RwasmtimeError::runtime(format!("failed to compile WASIp1 module: {err:#}"))
        })?;
        let mut linker = wasmtime::Linker::new(&self.engine);
        wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut WasiP1State| {
            &mut state.wasi
        })
        .map_err(|err| RwasmtimeError::runtime(format!("failed to link WASIp1 imports: {err}")))?;

        let (state, stdout_capture, stderr_capture) = build_wasi_p1_state(wasi, limits)?;
        let mut store = new_wasi_p1_store(&self.engine, state)?;
        let wall_clock = configure_store_limits(&mut store, limits)?;
        let instance = {
            let _instantiate_wall_time_guard = enter_wall_time_call(&mut store, &wall_clock);
            linker.instantiate(&mut store, &module).map_err(|err| {
                RwasmtimeError::runtime(format!("failed to instantiate WASIp1 module: {err:#}"))
            })?
        };
        let start = instance
            .get_typed_func::<(), ()>(&mut store, "_start")
            .map_err(|err| {
                RwasmtimeError::runtime(format!("failed to resolve WASIp1 `_start`: {err}"))
            })?;
        let _wall_time_guard = enter_wall_time_call(&mut store, &wall_clock);
        start
            .call(&mut store, ())
            .map_err(|err| RwasmtimeError::runtime(format!("WASIp1 `_start` failed: {err}")))?;

        Ok(WasiCommandOutput {
            stdout: stdout_capture
                .map(|pipe| pipe.contents().to_vec())
                .unwrap_or_default(),
            stderr: stderr_capture
                .map(|pipe| pipe.contents().to_vec())
                .unwrap_or_default(),
        })
    }
}

impl RuntimeSpec {
    pub fn build_wasmtime(self) -> Result<WasmtimeRuntime> {
        WasmtimeRuntime::new(self)
    }
}

fn build_config(spec: &RuntimeSpec) -> Result<wasmtime::Config> {
    spec.validate()?;
    validate_backend_features(&spec.features)?;

    let mut config = wasmtime::Config::new();
    config.strategy(match spec.compiler.strategy {
        CompilerStrategy::Auto => wasmtime::Strategy::Auto,
        CompilerStrategy::Cranelift => wasmtime::Strategy::Cranelift,
        CompilerStrategy::Winch => wasmtime::Strategy::Winch,
    });
    config.cranelift_opt_level(match spec.compiler.opt_level {
        OptLevel::None => wasmtime::OptLevel::None,
        OptLevel::Speed => wasmtime::OptLevel::Speed,
        OptLevel::SpeedAndSize => wasmtime::OptLevel::SpeedAndSize,
    });
    config.parallel_compilation(spec.compiler.parallel);
    config.consume_fuel(true);
    #[cfg(target_has_atomic = "64")]
    config.epoch_interruption(true);

    config.wasm_component_model(spec.features.component_model);
    config.wasm_component_model_async(spec.features.component_model_async);
    config.wasm_simd(spec.features.simd);
    config.wasm_relaxed_simd(spec.features.relaxed_simd);
    config.relaxed_simd_deterministic(spec.features.relaxed_simd_deterministic);
    config.wasm_bulk_memory(spec.features.bulk_memory);
    config.wasm_multi_memory(spec.features.multi_memory);
    config.wasm_memory64(spec.features.memory64);
    config.wasm_threads(spec.features.threads);
    config.wasm_exceptions(spec.features.exceptions);

    Ok(config)
}

fn validate_backend_features(features: &FeatureSpec) -> Result<()> {
    if features.legacy_exceptions {
        return Err(RwasmtimeError::invalid_argument(
            "legacy_exceptions are not supported by this Wasmtime backend build",
        ));
    }
    if features.gc {
        return Err(RwasmtimeError::invalid_argument(
            "gc proposal support is not exposed by this Rwasmtime backend yet",
        ));
    }
    Ok(())
}

fn describe_component_item(
    name: &str,
    item: wasmtime::component::types::ComponentItem,
) -> ComponentItem {
    match item {
        wasmtime::component::types::ComponentItem::ComponentFunc(func) => ComponentItem {
            name: name.to_string(),
            interface: None,
            kind: ComponentItemKind::Function,
            params_schema: join_nonempty(func.params().map(|(name, ty)| {
                if name.is_empty() {
                    component_type_label(&ty).to_string()
                } else {
                    format!("{name}: {}", component_type_label(&ty))
                }
            })),
            results_schema: join_nonempty(
                func.results()
                    .map(|ty| component_type_label(&ty).to_string()),
            ),
        },
        wasmtime::component::types::ComponentItem::CoreFunc(func) => ComponentItem {
            name: name.to_string(),
            interface: None,
            kind: ComponentItemKind::Function,
            params_schema: join_nonempty(func.params().map(|ty| format!("{ty:?}"))),
            results_schema: join_nonempty(func.results().map(|ty| format!("{ty:?}"))),
        },
        wasmtime::component::types::ComponentItem::Resource(_) => ComponentItem {
            name: name.to_string(),
            interface: None,
            kind: ComponentItemKind::Resource,
            params_schema: None,
            results_schema: None,
        },
        wasmtime::component::types::ComponentItem::Component(_) => ComponentItem {
            name: name.to_string(),
            interface: None,
            kind: ComponentItemKind::World,
            params_schema: None,
            results_schema: None,
        },
        wasmtime::component::types::ComponentItem::Module(_)
        | wasmtime::component::types::ComponentItem::ComponentInstance(_)
        | wasmtime::component::types::ComponentItem::Type(_) => ComponentItem {
            name: name.to_string(),
            interface: None,
            kind: ComponentItemKind::Interface,
            params_schema: None,
            results_schema: None,
        },
    }
}

fn join_nonempty(values: impl Iterator<Item = String>) -> Option<String> {
    let values: Vec<String> = values.collect();
    if values.is_empty() {
        None
    } else {
        Some(values.join(", "))
    }
}

fn component_type_label(ty: &wasmtime::component::types::Type) -> &'static str {
    match ty {
        wasmtime::component::types::Type::Bool => "bool",
        wasmtime::component::types::Type::S8 => "s8",
        wasmtime::component::types::Type::U8 => "u8",
        wasmtime::component::types::Type::S16 => "s16",
        wasmtime::component::types::Type::U16 => "u16",
        wasmtime::component::types::Type::S32 => "s32",
        wasmtime::component::types::Type::U32 => "u32",
        wasmtime::component::types::Type::S64 => "s64",
        wasmtime::component::types::Type::U64 => "u64",
        wasmtime::component::types::Type::Float32 => "float32",
        wasmtime::component::types::Type::Float64 => "float64",
        wasmtime::component::types::Type::Char => "char",
        wasmtime::component::types::Type::String => "string",
        wasmtime::component::types::Type::List(_) => "list",
        wasmtime::component::types::Type::Record(_) => "record",
        wasmtime::component::types::Type::Tuple(_) => "tuple",
        wasmtime::component::types::Type::Variant(_) => "variant",
        wasmtime::component::types::Type::Enum(_) => "enum",
        wasmtime::component::types::Type::Option(_) => "option",
        wasmtime::component::types::Type::Result(_) => "result",
        wasmtime::component::types::Type::Flags(_) => "flags",
        wasmtime::component::types::Type::Own(_) => "own",
        wasmtime::component::types::Type::Borrow(_) => "borrow",
        wasmtime::component::types::Type::Future(_) => "future",
        wasmtime::component::types::Type::Stream(_) => "stream",
        wasmtime::component::types::Type::ErrorContext => "error-context",
    }
}

#[cfg(feature = "wasi")]
fn validate_supported_wasi_backend(wasi: &WasiSpec) -> Result<()> {
    if wasi.network {
        return Err(RwasmtimeError::not_implemented(
            "WASI network capability is not implemented by the backend adapter yet",
        ));
    }
    if wasi.clocks {
        return Err(RwasmtimeError::not_implemented(
            "explicit WASI clock policy is not implemented by the backend adapter yet",
        ));
    }
    if wasi.random {
        return Err(RwasmtimeError::not_implemented(
            "explicit WASI random policy is not implemented by the backend adapter yet",
        ));
    }
    if matches!(wasi.stdin, StdioMode::File) {
        return Err(RwasmtimeError::not_implemented(
            "WASI stdin file mode is not implemented by the backend adapter yet",
        ));
    }
    Ok(())
}

#[cfg(feature = "wasi")]
fn build_wasi_p1_state(
    wasi: &WasiSpec,
    limits: CoreExecutionLimits,
) -> Result<(
    WasiP1State,
    Option<wasmtime_wasi::p2::pipe::MemoryOutputPipe>,
    Option<wasmtime_wasi::p2::pipe::MemoryOutputPipe>,
)> {
    let mut builder = wasmtime_wasi::WasiCtxBuilder::new();
    builder.allow_blocking_current_thread(true);

    for arg in &wasi.args {
        builder.arg(arg);
    }
    for (key, value) in &wasi.env {
        builder.env(key, value);
    }
    for preopen in &wasi.preopens {
        let dir_perms = if preopen.readonly {
            wasmtime_wasi::DirPerms::READ
        } else {
            wasmtime_wasi::DirPerms::all()
        };
        let file_perms = if preopen.readonly {
            wasmtime_wasi::FilePerms::READ
        } else {
            wasmtime_wasi::FilePerms::all()
        };
        builder
            .preopened_dir(&preopen.host, &preopen.guest, dir_perms, file_perms)
            .map_err(|err| {
                RwasmtimeError::runtime(format!(
                    "failed to preopen `{}` as `{}`: {err}",
                    preopen.host, preopen.guest
                ))
            })?;
    }

    match wasi.stdin {
        StdioMode::Empty | StdioMode::Discard | StdioMode::Capture => {}
        StdioMode::Inherit => {
            builder.inherit_stdin();
        }
        StdioMode::String => {
            let bytes = wasi.stdin_bytes.clone().unwrap_or_default();
            builder.stdin(wasmtime_wasi::p2::pipe::MemoryInputPipe::new(bytes));
        }
        StdioMode::File => unreachable!("validated by validate_supported_wasi_backend"),
    }

    let stdout_capture = match wasi.stdout {
        StdioMode::Capture => {
            let pipe = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(1024 * 1024);
            builder.stdout(pipe.clone());
            Some(pipe)
        }
        StdioMode::Inherit => {
            builder.inherit_stdout();
            None
        }
        StdioMode::File => {
            return Err(RwasmtimeError::not_implemented(
                "WASI stdout file mode is not implemented by the backend adapter yet",
            ));
        }
        StdioMode::Empty | StdioMode::Discard | StdioMode::String => None,
    };

    let stderr_capture = match wasi.stderr {
        StdioMode::Capture => {
            let pipe = wasmtime_wasi::p2::pipe::MemoryOutputPipe::new(1024 * 1024);
            builder.stderr(pipe.clone());
            Some(pipe)
        }
        StdioMode::Inherit => {
            builder.inherit_stderr();
            None
        }
        StdioMode::File => {
            return Err(RwasmtimeError::not_implemented(
                "WASI stderr file mode is not implemented by the backend adapter yet",
            ));
        }
        StdioMode::Empty | StdioMode::Discard | StdioMode::String => None,
    };

    Ok((
        WasiP1State {
            wasi: builder.build_p1(),
            resource_limits: RuntimeStoreLimits::new(limits)?,
        },
        stdout_capture,
        stderr_capture,
    ))
}

impl RwasmtimeError {
    pub fn runtime(message: impl Into<String>) -> Self {
        Self {
            kind: RwasmtimeErrorKind::Runtime,
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CompilerSpec, FeatureSpec};

    const ADD_WAT: &str = r#"
        (module
          (func (export "add") (param i32 i32) (result i32)
            local.get 0
            local.get 1
            i32.add))
    "#;

    #[cfg(feature = "wasi")]
    const ECHO_STDIN_WASI_WAT: &str = r#"
        (module
          (import "wasi_snapshot_preview1" "fd_read"
            (func $fd_read (param i32 i32 i32 i32) (result i32)))
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (memory (export "memory") 1)
          (func (export "_start")
            (i32.store (i32.const 0) (i32.const 32))
            (i32.store (i32.const 4) (i32.const 64))
            (drop (call $fd_read (i32.const 0) (i32.const 0) (i32.const 1) (i32.const 24)))
            (i32.store (i32.const 8) (i32.const 32))
            (i32.store (i32.const 12) (i32.load (i32.const 24)))
            (drop (call $fd_write (i32.const 1) (i32.const 8) (i32.const 1) (i32.const 28)))))
    "#;

    const MEMORY_WAT: &str = r#"
        (module
          (memory (export "memory") 1 2)
          (data (i32.const 0) "abc")
          (func (export "load8") (param i32) (result i32)
            local.get 0
            i32.load8_u)
          (func (export "store8") (param i32 i32)
            local.get 0
            local.get 1
            i32.store8))
    "#;

    const GROW_MEMORY_WAT: &str = r#"
        (module
          (memory (export "memory") 1 2)
          (func (export "grow_memory") (param i32) (result i32)
            local.get 0
            memory.grow))
    "#;

    const INITIAL_TABLE_WAT: &str = r#"
        (module
          (table (export "table") 2 2 funcref))
    "#;

    const GROW_TABLE_WAT: &str = r#"
        (module
          (table (export "table") 1 2 funcref)
          (func (export "grow_table") (param i32) (result i32)
            ref.null func
            local.get 0
            table.grow))
    "#;

    const CALLBACK_WAT: &str = r#"
        (module
          (import "r" "add_one" (func $add_one (param i32) (result i32)))
          (func (export "run") (param i32) (result i32)
            local.get 0
            call $add_one
            i32.const 40
            i32.add))
    "#;

    fn modern_exception_tag_module() -> Vec<u8> {
        vec![
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
            // Type section: tag payload type `(i32) -> ()` and function `() -> i32`.
            0x01, 0x09, 0x02, 0x60, 0x01, 0x7f, 0x00, 0x60, 0x00, 0x01, 0x7f,
            // Function section: one function with type index 1.
            0x03, 0x02, 0x01, 0x01,
            // Tag section: one modern exception tag with type index 0.
            0x0d, 0x03, 0x01, 0x00, 0x00,
            // Export section: export the function as `answer`.
            0x07, 0x0a, 0x01, 0x06, 0x61, 0x6e, 0x73, 0x77, 0x65, 0x72, 0x00, 0x00,
            // Code section: `i32.const 42`.
            0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b,
        ]
    }

    const COMPONENT_IMPORT_WAT: &str = r#"
        (component
          (import "host-add" (func (param "x" s32) (param "y" s32) (result s32))))
    "#;

    const COMPONENT_EXPORT_WAT: &str = r#"
        (component
          (core module $m
            (func (export "answer") (result i32)
              i32.const 42))
          (core instance $i (instantiate $m))
          (func $answer (result s32) (canon lift (core func $i "answer")))
          (export "answer" (func $answer)))
    "#;

    #[cfg(feature = "wasi")]
    const PREOPEN_PROBE_WASI_WAT: &str = r#"
        (module
          (import "wasi_snapshot_preview1" "fd_prestat_get"
            (func $fd_prestat_get (param i32 i32) (result i32)))
          (import "wasi_snapshot_preview1" "fd_write"
            (func $fd_write (param i32 i32 i32 i32) (result i32)))
          (memory (export "memory") 1)
          (data (i32.const 32) "deny\n")
          (data (i32.const 48) "grant\n")
          (func (export "_start")
            (local $errno i32)
            (local.set $errno (call $fd_prestat_get (i32.const 3) (i32.const 0)))
            (if (i32.eqz (local.get $errno))
              (then
                (i32.store (i32.const 8) (i32.const 48))
                (i32.store (i32.const 12) (i32.const 6)))
              (else
                (i32.store (i32.const 8) (i32.const 32))
                (i32.store (i32.const 12) (i32.const 5))))
            (drop (call $fd_write (i32.const 1) (i32.const 8) (i32.const 1) (i32.const 24)))))
    "#;

    fn test_runtime() -> WasmtimeRuntime {
        RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build with Cranelift")
    }

    #[test]
    fn cranelift_runtime_compiles_instantiates_and_calls_core_module() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build with Cranelift");

        let args = [wasmtime::Val::I32(20), wasmtime::Val::I32(22)];
        let mut results = [wasmtime::Val::I32(0)];
        runtime
            .call_core_export(
                ADD_WAT,
                "add",
                &args,
                &mut results,
                CoreExecutionLimits::none(),
            )
            .expect("real Wasm call should succeed");
        assert_eq!(results[0].unwrap_i32(), 42);
        assert_eq!(
            runtime.spec().compiler.strategy,
            CompilerStrategy::Cranelift
        );
    }

    #[test]
    fn backend_supports_modern_wasm_exceptions_when_enabled() {
        let bytes = modern_exception_tag_module();
        let default_runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("default runtime should build");
        let err = default_runtime
            .compile_core(&bytes)
            .expect_err("tag section requires exceptions to be enabled");
        assert!(
            err.message.contains("exceptions proposal not enabled"),
            "{}",
            err.message
        );

        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false).exceptions(true))
            .build_wasmtime()
            .expect("modern Wasm exceptions should be supported by this backend build");
        let module = runtime
            .compile_core(&bytes)
            .expect("modern exception tag module should compile when exceptions are enabled");
        let exports = module.exports();
        assert!(exports.iter().any(|item| item.name == "answer"));
        let mut instance = runtime
            .instantiate_core_module(&module, CoreExecutionLimits::none())
            .expect("tag-only module should instantiate");
        let mut results = [wasmtime::Val::I32(0)];
        instance.call_export("answer", &[], &mut results).unwrap();
        assert_eq!(results[0].unwrap_i32(), 42);
    }

    #[test]
    fn backend_still_rejects_legacy_wasm_exceptions() {
        let err = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(
                FeatureSpec::new()
                    .component_model(false)
                    .exceptions(true)
                    .legacy_exceptions(true),
            )
            .build_wasmtime()
            .expect_err("legacy exceptions remain unsupported by this backend");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("legacy_exceptions"));
    }

    #[test]
    fn component_introspection_reports_imports_and_exports() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(true))
            .build_wasmtime()
            .expect("component model runtime should build");

        let imports = runtime
            .component_imports(COMPONENT_IMPORT_WAT)
            .expect("component imports should parse");
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "host-add");
        assert_eq!(imports[0].kind, ComponentItemKind::Function);
        assert_eq!(imports[0].params_schema.as_deref(), Some("x: s32, y: s32"));
        assert_eq!(imports[0].results_schema.as_deref(), Some("s32"));

        let exports = runtime
            .component_exports(COMPONENT_EXPORT_WAT)
            .expect("component exports should parse");
        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "answer");
        assert_eq!(exports[0].kind, ComponentItemKind::Function);
        assert_eq!(exports[0].results_schema.as_deref(), Some("s32"));
    }

    #[test]
    fn core_instance_persists_memory_state_and_grows_memory() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build with Cranelift");
        let mut instance = runtime
            .instantiate_core(MEMORY_WAT, CoreExecutionLimits::none())
            .expect("core module should instantiate once");

        assert_eq!(instance.memory_size_pages("memory").unwrap(), 1);
        assert_eq!(instance.memory_read("memory", 0, 3).unwrap(), b"abc");
        instance.memory_write("memory", 1, b"Z").unwrap();
        assert_eq!(instance.memory_read("memory", 0, 3).unwrap(), b"aZc");

        let args = [wasmtime::Val::I32(2), wasmtime::Val::I32(b'Q' as i32)];
        instance.call_export("store8", &args, &mut []).unwrap();
        let mut results = [wasmtime::Val::I32(0)];
        instance
            .call_export("load8", &[wasmtime::Val::I32(2)], &mut results)
            .unwrap();
        assert_eq!(results[0].unwrap_i32(), b'Q' as i32);
        assert_eq!(instance.memory_grow_pages("memory", 1).unwrap(), 1);
        assert_eq!(instance.memory_size_pages("memory").unwrap(), 2);
    }

    #[test]
    fn store_memory_limit_rejects_initial_memory_above_limit() {
        let runtime = test_runtime();
        let limits = CoreExecutionLimits::none().resource_limits(Some(260), None, None);
        let err = match runtime.instantiate_core(MEMORY_WAT, limits) {
            Ok(_) => panic!("initial memory must exceed the configured byte limit"),
            Err(err) => err,
        };
        assert!(
            err.message.contains("memory limit exceeded"),
            "{}",
            err.message
        );
        assert!(err.message.contains("65536"), "{}", err.message);
        assert!(err.message.contains("260"), "{}", err.message);
    }

    #[test]
    fn store_memory_limit_rejects_guest_internal_memory_grow() {
        let runtime = test_runtime();
        let limits = CoreExecutionLimits::none().resource_limits(Some(65_536), None, None);
        let mut instance = runtime
            .instantiate_core(GROW_MEMORY_WAT, limits)
            .expect("one page initial memory is within the limit");
        let args = [wasmtime::Val::I32(1)];
        let mut results = [wasmtime::Val::I32(0)];
        let err = instance
            .call_export("grow_memory", &args, &mut results)
            .expect_err("guest memory.grow must be rejected by the store limiter");
        assert!(
            err.message.contains("memory limit exceeded"),
            "{}",
            err.message
        );
        assert!(err.message.contains("131072"), "{}", err.message);
        assert_eq!(instance.memory_size_pages("memory").unwrap(), 1);
    }

    #[test]
    fn store_table_limit_rejects_initial_table_above_limit() {
        let runtime = test_runtime();
        let limits = CoreExecutionLimits::none().resource_limits(None, Some(1), None);
        let err = match runtime.instantiate_core(INITIAL_TABLE_WAT, limits) {
            Ok(_) => panic!("initial table must exceed the configured element limit"),
            Err(err) => err,
        };
        assert!(
            err.message.contains("table element limit exceeded"),
            "{}",
            err.message
        );
        assert!(err.message.contains("2"), "{}", err.message);
        assert!(err.message.contains("1"), "{}", err.message);
    }

    #[test]
    fn store_table_limit_rejects_guest_internal_table_grow() {
        let runtime = test_runtime();
        let limits = CoreExecutionLimits::none().resource_limits(None, Some(1), None);
        let mut instance = runtime
            .instantiate_core(GROW_TABLE_WAT, limits)
            .expect("one table element is within the configured limit");
        let args = [wasmtime::Val::I32(1)];
        let mut results = [wasmtime::Val::I32(0)];
        let err = instance
            .call_export("grow_table", &args, &mut results)
            .expect_err("guest table.grow must be rejected by the store limiter");
        assert!(
            err.message.contains("table element limit exceeded"),
            "{}",
            err.message
        );
        assert!(err.message.contains("2"), "{}", err.message);
    }

    #[test]
    fn store_instance_limit_zero_rejects_instantiation() {
        let runtime = test_runtime();
        let limits = CoreExecutionLimits::none().resource_limits(None, None, Some(0));
        let err = match runtime.instantiate_core(ADD_WAT, limits) {
            Ok(_) => panic!("zero instance limit must reject instantiation"),
            Err(err) => err,
        };
        assert!(
            err.message.contains("resource limit exceeded"),
            "{}",
            err.message
        );
    }

    #[test]
    fn compiled_core_module_instantiates_with_host_callback_imports() {
        use std::sync::Arc;

        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build with Cranelift");
        let module = runtime
            .compile_core(CALLBACK_WAT)
            .expect("callback module should compile");
        let imports = module.func_imports();
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].module, "r");
        assert_eq!(imports[0].name, "add_one");
        let items = module.imports();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].module.as_deref(), Some("r"));
        assert_eq!(items[0].name, "add_one");
        assert_eq!(items[0].kind, "function");
        assert_eq!(items[0].params, vec!["i32"]);
        assert_eq!(items[0].results, vec!["i32"]);

        let host = CoreHostFunc::new(
            imports[0].module.clone(),
            imports[0].name.clone(),
            imports[0].params.clone(),
            imports[0].results.clone(),
            Arc::new(|name, args, results| {
                assert_eq!(name, "r::add_one");
                results[0] = wasmtime::Val::I32(args[0].unwrap_i32() + 1);
                Ok(())
            }),
        );
        let mut instance = runtime
            .instantiate_core_module_with_host_funcs(
                &module,
                vec![host],
                CoreExecutionLimits::none(),
            )
            .expect("module with host callback should instantiate");
        let mut results = [wasmtime::Val::I32(0)];
        instance
            .call_export("run", &[wasmtime::Val::I32(1)], &mut results)
            .unwrap();
        assert_eq!(results[0].unwrap_i32(), 42);
    }

    #[test]
    fn compiled_core_module_can_instantiate_separate_stateful_instances() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build with Cranelift");
        let module = runtime
            .compile_core(MEMORY_WAT)
            .expect("core module should compile once");
        let exports = module.exports();
        assert!(exports.iter().any(|item| item.name == "load8"
            && item.kind == "function"
            && item.params == vec!["i32"]
            && item.results == vec!["i32"]));
        let memory = exports
            .iter()
            .find(|item| item.name == "memory")
            .expect("memory export should be described");
        assert_eq!(memory.kind, "memory");
        assert_eq!(memory.minimum.as_deref(), Some("1"));
        assert_eq!(memory.maximum.as_deref(), Some("2"));
        assert_eq!(memory.shared, Some(false));
        assert_eq!(memory.memory64, Some(false));
        let mut a = runtime
            .instantiate_core_module(&module, CoreExecutionLimits::none())
            .expect("first instance");
        let mut b = runtime
            .instantiate_core_module(&module, CoreExecutionLimits::none())
            .expect("second instance");

        a.memory_write("memory", 0, b"xyz").unwrap();
        assert_eq!(a.memory_read("memory", 0, 3).unwrap(), b"xyz");
        assert_eq!(b.memory_read("memory", 0, 3).unwrap(), b"abc");
    }

    #[test]
    fn compiled_core_module_serializes_and_deserializes_with_same_runtime() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build with Cranelift");
        let module = runtime
            .compile_core(ADD_WAT)
            .expect("core module should compile once");
        let bytes = module
            .serialize()
            .expect("compiled module should serialize");
        assert!(!bytes.is_empty());
        let module = runtime
            .deserialize_core(&bytes)
            .expect("serialized module should reload");
        let mut instance = runtime
            .instantiate_core_module(&module, CoreExecutionLimits::none())
            .expect("deserialized instance");
        let mut results = [wasmtime::Val::I32(0)];
        instance
            .call_export(
                "add",
                &[wasmtime::Val::I32(2), wasmtime::Val::I32(40)],
                &mut results,
            )
            .unwrap();
        assert_eq!(results[0].unwrap_i32(), 42);
    }

    #[test]
    fn backend_preserves_existing_feature_validation() {
        let err = RuntimeSpec::new()
            .features(FeatureSpec::new().relaxed_simd_deterministic(true))
            .build_wasmtime()
            .expect_err("deterministic relaxed SIMD still requires relaxed SIMD");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("relaxed_simd_deterministic"));
    }

    #[test]
    fn backend_rejects_unsupported_gc_feature_until_enabled_deliberately() {
        let mut features = FeatureSpec::new();
        features.gc = true;
        let err = RuntimeSpec::new()
            .features(features)
            .build_wasmtime()
            .expect_err("GC must not be silently enabled without the Wasmtime GC feature set");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("GC") || err.message.contains("gc"));
    }

    #[cfg(feature = "wasi")]
    #[test]
    fn wasi_preview1_command_captures_stdout_from_string_stdin() {
        use crate::wasi::WasiSpec;

        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build for WASI");
        let wasi = WasiSpec::new().stdin_text("hello from wasi");
        let output = runtime
            .run_wasi_p1_command(ECHO_STDIN_WASI_WAT, &wasi, CoreExecutionLimits::none())
            .expect("WASIp1 stdin/stdout command should run");

        assert_eq!(output.stdout, b"hello from wasi");
        assert!(output.stderr.is_empty());
    }

    #[cfg(feature = "wasi")]
    #[test]
    fn wasi_preview1_command_captures_binary_stdin_bytes() {
        use crate::wasi::WasiSpec;

        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build for WASI");
        let wasi = WasiSpec::new().stdin_bytes(vec![b'A', 0, b'B']);
        let output = runtime
            .run_wasi_p1_command(ECHO_STDIN_WASI_WAT, &wasi, CoreExecutionLimits::none())
            .expect("WASIp1 binary stdin command should run");

        assert_eq!(output.stdout, &[b'A', 0, b'B']);
        assert!(output.stderr.is_empty());
    }

    #[cfg(feature = "wasi")]
    #[test]
    fn compiled_core_module_instantiates_with_wasi_p1_imports() {
        use crate::wasi::WasiSpec;

        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build for WASI");
        let module = runtime
            .compile_core(ECHO_STDIN_WASI_WAT)
            .expect("WASI core module should compile");
        let wasi = WasiSpec::new().stdin_text("hello from low-level wasi");
        let mut instance = runtime
            .instantiate_core_module_wasi_p1(&module, &wasi, CoreExecutionLimits::none())
            .expect("compiled WASIp1 module should instantiate through explicit linker");
        instance
            .call_export("_start", &[], &mut [])
            .expect("WASIp1 _start should run");
        let output = instance.wasi_output();
        assert_eq!(output.stdout, b"hello from low-level wasi");
        assert!(output.stderr.is_empty());
    }

    #[cfg(feature = "wasi")]
    #[test]
    fn wasi_preview1_preopen_is_deny_by_default_and_explicit_when_configured() {
        use crate::wasi::WasiSpec;
        use std::fs;
        use std::time::{SystemTime, UNIX_EPOCH};

        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(false))
            .build_wasmtime()
            .expect("real Wasmtime runtime should build for WASI");

        let denied = runtime
            .run_wasi_p1_command(
                PREOPEN_PROBE_WASI_WAT,
                &WasiSpec::new(),
                CoreExecutionLimits::none(),
            )
            .expect("WASIp1 command should run without preopens");
        assert_eq!(denied.stdout, b"deny\n");

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let host_dir = std::env::temp_dir().join(format!("rwasmtime-wasi-preopen-{unique}"));
        fs::create_dir_all(&host_dir).expect("test preopen directory should be created");
        let host_dir_string = host_dir.to_string_lossy().into_owned();
        let wasi = WasiSpec::new().preopen("/data", host_dir_string, true);
        let granted = runtime
            .run_wasi_p1_command(PREOPEN_PROBE_WASI_WAT, &wasi, CoreExecutionLimits::none())
            .expect("WASIp1 command should run with explicit preopen");
        assert_eq!(granted.stdout, b"grant\n");
        fs::remove_dir_all(&host_dir).ok();
    }
}
