use crate::callbacks::CallbackSet;
use crate::limits::Limits;
use crate::wasi::WasiSpec;
use crate::{Runtime, RuntimeSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RwasmtimeErrorKind {
    InvalidArgument,
    NotImplemented,
    Runtime,
    AotIncompatible,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RwasmtimeError {
    pub kind: RwasmtimeErrorKind,
    pub message: String,
}

impl RwasmtimeError {
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            kind: RwasmtimeErrorKind::InvalidArgument,
            message: message.into(),
        }
    }

    pub fn not_implemented(feature: impl Into<String>) -> Self {
        Self {
            kind: RwasmtimeErrorKind::NotImplemented,
            message: feature.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, RwasmtimeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Auto,
    Module,
    Component,
    Artifact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayDType {
    F64,
    F32,
    I32,
    I64,
    U8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayLayout {
    ColumnMajor,
    RowMajor,
    Strided,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayTransport {
    Component,
    Memory,
    Arena,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayPolicy {
    pub default_dtype: ArrayDType,
    pub layout: ArrayLayout,
    pub transport: ArrayTransport,
}

impl ArrayPolicy {
    pub fn new() -> Self {
        Self {
            default_dtype: ArrayDType::F64,
            layout: ArrayLayout::ColumnMajor,
            transport: ArrayTransport::Arena,
        }
    }

    pub fn default_dtype(mut self, value: ArrayDType) -> Self {
        self.default_dtype = value;
        self
    }
    pub fn layout(mut self, value: ArrayLayout) -> Self {
        self.layout = value;
        self
    }
    pub fn transport(mut self, value: ArrayTransport) -> Self {
        self.transport = value;
        self
    }
}

impl Default for ArrayPolicy {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitSpec {
    pub path: String,
    pub world: Option<String>,
    pub validate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppSpec {
    pub source: String,
    pub kind: SourceKind,
    pub runtime: Option<RuntimeSpec>,
    pub wasi: Option<WasiSpec>,
    pub limits: Option<Limits>,
    pub callbacks: Option<CallbackSet>,
    pub arrays: ArrayPolicy,
    pub wit: Option<WitSpec>,
}

impl AppSpec {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            kind: SourceKind::Auto,
            runtime: None,
            wasi: None,
            limits: None,
            callbacks: None,
            arrays: ArrayPolicy::default(),
            wit: None,
        }
    }

    pub fn module(mut self) -> Self {
        self.kind = SourceKind::Module;
        self
    }
    pub fn component(mut self) -> Self {
        self.kind = SourceKind::Component;
        self
    }
    pub fn artifact(mut self) -> Self {
        self.kind = SourceKind::Artifact;
        self
    }

    pub fn runtime(mut self, runtime: &Runtime) -> Self {
        self.runtime = Some(runtime.spec.clone());
        self
    }

    pub fn runtime_spec(mut self, runtime: RuntimeSpec) -> Self {
        self.runtime = Some(runtime);
        self
    }

    pub fn wasi(mut self, wasi: WasiSpec) -> Self {
        self.wasi = Some(wasi);
        self
    }
    pub fn limits(mut self, limits: Limits) -> Self {
        self.limits = Some(limits);
        self
    }
    pub fn callbacks(mut self, callbacks: CallbackSet) -> Self {
        self.callbacks = Some(callbacks);
        self
    }
    pub fn arrays(mut self, arrays: ArrayPolicy) -> Self {
        self.arrays = arrays;
        self
    }

    pub fn wit(mut self, path: impl Into<String>, world: Option<String>, validate: bool) -> Self {
        self.wit = Some(WitSpec {
            path: path.into(),
            world,
            validate,
        });
        self
    }

    pub fn prepare(self) -> PreparedApp {
        PreparedApp { spec: self }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedApp {
    pub spec: AppSpec,
}

impl PreparedApp {
    pub fn new_session(&self) -> Session {
        Session {
            app: self.clone(),
            temp_arrays: Vec::new(),
        }
    }

    pub fn call(&self, export: &str, _args: Vec<Value>) -> Result<Value> {
        Err(RwasmtimeError::not_implemented(format!(
            "wt_call({export})"
        )))
    }

    pub fn call_async(&self, export: impl Into<String>, args: Vec<Value>) -> Job {
        Job {
            export: export.into(),
            args,
            state: JobState::Pending,
            result: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub app: PreparedApp,
    pub temp_arrays: Vec<TempArray>,
}

impl Session {
    pub fn exec(&mut self, export: &str, _args: Vec<Value>) -> Result<&mut Self> {
        Err(RwasmtimeError::not_implemented(format!(
            "wt_exec({export})"
        )))
    }

    pub fn call(&mut self, export: &str, _args: Vec<Value>) -> Result<Value> {
        Err(RwasmtimeError::not_implemented(format!(
            "wt_call({export})"
        )))
    }

    pub fn with_temp_array(mut self, name: impl Into<String>, dtype: Option<ArrayDType>) -> Self {
        self.temp_arrays.push(TempArray {
            name: name.into(),
            dtype,
        });
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempArray {
    pub name: String,
    pub dtype: Option<ArrayDType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    None,
    I32(i32),
    I64(i64),
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Pending,
    Done,
    Cancelled,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub export: String,
    pub args: Vec<Value>,
    pub state: JobState,
    pub result: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobPoll {
    pub done: bool,
    pub cancelled: bool,
    pub state: JobState,
}

impl Job {
    pub fn poll(&self) -> JobState {
        self.state
    }

    pub fn poll_status(&self) -> JobPoll {
        JobPoll {
            done: self.state == JobState::Done,
            cancelled: self.state == JobState::Cancelled,
            state: self.state,
        }
    }

    pub fn cancel(mut self) -> Self {
        self.state = JobState::Cancelled;
        self
    }

    pub fn result(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    pub fn await_result(&self) -> Result<Value> {
        match (&self.state, &self.result) {
            (JobState::Done, Some(value)) => Ok(value.clone()),
            (JobState::Cancelled, _) => Err(RwasmtimeError {
                kind: RwasmtimeErrorKind::InvalidArgument,
                message: "job was cancelled".to_string(),
            }),
            _ => Err(RwasmtimeError::not_implemented("wt_await")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callbacks::{CallbackPolicy, CallbackSet, CallbackSpec};
    use crate::config::{CompilerSpec, FeatureSpec};
    use crate::limits::Limits;
    use crate::wasi::{StdioMode, WasiSpec};

    #[test]
    fn app_pipeline_preserves_capability_objects() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift())
            .features(FeatureSpec::new().component_model(true))
            .build();
        let wasi = WasiSpec::new()
            .arg("--input")
            .preopen("/data", "/safe/data", true)
            .stdio(StdioMode::Empty, StdioMode::Capture, StdioMode::Capture);
        let limits = Limits::new()
            .memory_bytes(512 * 1024 * 1024)
            .wall_time_ms(5_000);
        let callbacks = CallbackSet::new().callback(
            CallbackSpec::component("rwasmtime:host/callbacks.log")
                .policy(CallbackPolicy::blocking_main_thread()),
        );

        let app = AppSpec::new("stats_plugin.component.wasm")
            .component()
            .runtime(&runtime)
            .wasi(wasi)
            .limits(limits)
            .callbacks(callbacks)
            .arrays(ArrayPolicy::new().transport(ArrayTransport::Arena))
            .prepare();

        assert_eq!(app.spec.kind, SourceKind::Component);
        assert!(app.spec.runtime.is_some());
        assert_eq!(app.spec.wasi.as_ref().map(|w| w.preopens.len()), Some(1));
        assert_eq!(
            app.spec.limits.as_ref().and_then(|l| l.wall_time_ms),
            Some(5_000)
        );
        assert_eq!(
            app.spec.callbacks.as_ref().map(|c| c.imports.len()),
            Some(1)
        );
    }

    #[test]
    fn execution_boundaries_fail_honestly_until_backend_lands() {
        let app = AppSpec::new("tool.wasm").module().prepare();
        let err = app
            .call("run", Vec::new())
            .expect_err("call should be pending backend");
        assert_eq!(err.kind, RwasmtimeErrorKind::NotImplemented);
        assert!(err.message.contains("wt_call(run)"));

        let job = app.call_async("fit", vec![Value::I32(1)]);
        assert_eq!(job.poll(), JobState::Pending);
        let status = job.poll_status();
        assert_eq!(status.state, JobState::Pending);
        assert!(!status.done);
        let await_err = job
            .await_result()
            .expect_err("await should be pending backend");
        assert_eq!(await_err.kind, RwasmtimeErrorKind::NotImplemented);
    }
}
