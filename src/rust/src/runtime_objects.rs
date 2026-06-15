use crate::app::{Result, RwasmtimeError, SourceKind, Value};
use crate::callbacks::CallbackSet;
use crate::limits::Limits;
use crate::wasi::WasiSpec;
use crate::{ArtifactMetadata, Runtime, RuntimeSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    pub runtime: RuntimeSpec,
    pub input: String,
    pub kind: SourceKind,
    pub metadata: ArtifactMetadata,
    pub aot_path: Option<String>,
}

impl Artifact {
    pub fn info(&self) -> ArtifactInfo {
        ArtifactInfo {
            input: self.input.clone(),
            kind: self.kind,
            metadata: self.metadata.clone(),
            aot_path: self.aot_path.clone(),
        }
    }

    pub fn compatible_with(&self, runtime: &Runtime) -> bool {
        self.metadata.compatible_with(runtime, self.kind)
    }

    pub fn aot_save(mut self, path: impl Into<String>) -> Self {
        self.aot_path = Some(path.into());
        self
    }

    pub fn instantiate(&self, store: Store, linker: Linker) -> Result<Instance> {
        if store.runtime != self.runtime || linker.runtime != self.runtime {
            return Err(RwasmtimeError {
                kind: crate::app::RwasmtimeErrorKind::InvalidArgument,
                message: "artifact, store, and linker must come from compatible runtime specs"
                    .to_string(),
            });
        }
        self.metadata
            .ensure_compatible_with_spec(&store.runtime, self.kind)?;
        Ok(Instance {
            artifact: self.clone(),
            store,
            linker,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactInfo {
    pub input: String,
    pub kind: SourceKind,
    pub metadata: ArtifactMetadata,
    pub aot_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Store {
    pub runtime: RuntimeSpec,
    pub limits: Option<Limits>,
    pub wasi: Option<WasiSpec>,
    pub callbacks: Option<CallbackSet>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Linker {
    pub runtime: RuntimeSpec,
    pub wasi: Option<WasiSpec>,
    pub callbacks: Option<CallbackSet>,
}

impl Linker {
    pub fn link_wasi(mut self, wasi: WasiSpec) -> Self {
        self.wasi = Some(wasi);
        self
    }

    pub fn link_callbacks(mut self, callbacks: CallbackSet) -> Self {
        self.callbacks = Some(callbacks);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub artifact: Artifact,
    pub store: Store,
    pub linker: Linker,
}

impl Instance {
    pub fn call(&self, export: &str, _args: Vec<Value>) -> Result<Value> {
        Err(RwasmtimeError::not_implemented(format!(
            "wt_call({export})"
        )))
    }

    pub fn memory(&self, name: impl Into<String>) -> Memory {
        Memory { name: name.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Memory {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryDType {
    U8,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    V128,
}

impl MemoryDType {
    pub fn byte_width(self) -> u64 {
        match self {
            MemoryDType::U8 => 1,
            MemoryDType::I32 | MemoryDType::U32 | MemoryDType::F32 => 4,
            MemoryDType::I64 | MemoryDType::U64 | MemoryDType::F64 => 8,
            MemoryDType::V128 => 16,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLayout {
    Contiguous,
    RowMajor,
    ColumnMajor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySpan {
    pub ptr: u64,
    pub len: u64,
    pub dtype: MemoryDType,
    pub dim: Option<Vec<u64>>,
    pub layout: MemoryLayout,
}

impl MemorySpan {
    pub fn new(ptr: u64, len: u64, dtype: MemoryDType) -> Self {
        Self {
            ptr,
            len,
            dtype,
            dim: None,
            layout: MemoryLayout::Contiguous,
        }
    }

    pub fn dim(mut self, value: Vec<u64>) -> Self {
        self.dim = Some(value);
        self
    }
    pub fn layout(mut self, value: MemoryLayout) -> Self {
        self.layout = value;
        self
    }

    pub fn end(&self) -> Result<u64> {
        self.ptr
            .checked_add(self.len)
            .ok_or_else(|| RwasmtimeError::invalid_argument("memory span overflows address space"))
    }

    pub fn element_count(&self) -> Option<u64> {
        product_dims(self.dim.as_deref())
    }

    pub fn expected_len_bytes(&self) -> Option<u64> {
        self.element_count()?.checked_mul(self.dtype.byte_width())
    }

    pub fn validate_shape(&self) -> Result<()> {
        let width = self.dtype.byte_width();
        if self.len % width != 0 {
            return Err(RwasmtimeError::invalid_argument(format!(
                "memory span length {} is not a multiple of {} for {:?}",
                self.len, width, self.dtype
            )));
        }
        if let Some(expected) = self.expected_len_bytes() {
            if expected != self.len {
                return Err(RwasmtimeError::invalid_argument(format!(
                    "memory span dim implies {expected} bytes but span length is {} bytes",
                    self.len
                )));
            }
        }
        Ok(())
    }

    pub fn validate_bounds(&self, memory_size_bytes: u64) -> Result<()> {
        self.validate_shape()?;
        let end = self.end()?;
        if end > memory_size_bytes {
            return Err(RwasmtimeError::invalid_argument(format!(
                "memory span [{}, {}) exceeds memory size {memory_size_bytes}",
                self.ptr, end
            )));
        }
        Ok(())
    }
}

fn product_dims(dim: Option<&[u64]>) -> Option<u64> {
    let dim = dim?;
    dim.iter()
        .try_fold(1_u64, |acc, value| acc.checked_mul(*value))
}

impl Memory {
    pub fn size(&self) -> Result<u64> {
        Err(RwasmtimeError::not_implemented("wt_memory_size"))
    }

    pub fn grow(&self, _pages: u64) -> Result<u64> {
        Err(RwasmtimeError::not_implemented("wt_memory_grow"))
    }

    pub fn read(&self, span: &MemorySpan) -> Result<Vec<u8>> {
        span.validate_shape()?;
        Err(RwasmtimeError::not_implemented("wt_memory_read"))
    }

    pub fn write(&self, span: &MemorySpan, bytes: &[u8]) -> Result<()> {
        span.validate_shape()?;
        if bytes.len() as u64 != span.len {
            return Err(RwasmtimeError::invalid_argument(format!(
                "memory write payload has {} bytes but span length is {} bytes",
                bytes.len(),
                span.len
            )));
        }
        Err(RwasmtimeError::not_implemented("wt_memory_write"))
    }
}

impl Runtime {
    pub fn compile(&self, input: impl Into<String>, kind: SourceKind) -> Artifact {
        Artifact {
            runtime: self.spec.clone(),
            input: input.into(),
            kind,
            metadata: ArtifactMetadata::from_runtime(self, kind),
            aot_path: None,
        }
    }

    pub fn aot_load(&self, path: impl Into<String>, _validate: bool) -> Artifact {
        Artifact {
            runtime: self.spec.clone(),
            input: path.into(),
            kind: SourceKind::Artifact,
            metadata: ArtifactMetadata::from_runtime(self, SourceKind::Artifact),
            aot_path: None,
        }
    }

    pub fn store(
        &self,
        limits: Option<Limits>,
        wasi: Option<WasiSpec>,
        callbacks: Option<CallbackSet>,
    ) -> Store {
        Store {
            runtime: self.spec.clone(),
            limits,
            wasi,
            callbacks,
        }
    }

    pub fn linker(&self) -> Linker {
        Linker {
            runtime: self.spec.clone(),
            wasi: None,
            callbacks: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::RwasmtimeErrorKind;
    use crate::callbacks::{CallbackSet, CallbackSpec};
    use crate::config::{CompilerSpec, FeatureSpec, RuntimeSpec};
    use crate::limits::Limits;
    use crate::wasi::WasiSpec;

    #[test]
    fn low_level_pipeline_preserves_runtime_compatibility() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift())
            .features(FeatureSpec::new().component_model(true).simd(true))
            .build();
        let wasi = WasiSpec::new().arg("--version");
        let callbacks = CallbackSet::new().callback(CallbackSpec::core("r", "score_f64"));
        let artifact = runtime
            .compile("add.wasm", SourceKind::Module)
            .aot_save("add.cwasm");
        let store = runtime.store(
            Some(Limits::new().memory_bytes(64 * 1024 * 1024)),
            Some(wasi.clone()),
            None,
        );
        let linker = runtime.linker().link_wasi(wasi).link_callbacks(callbacks);
        let instance = artifact
            .instantiate(store, linker)
            .expect("runtime specs match");

        assert_eq!(
            instance.artifact.info().aot_path.as_deref(),
            Some("add.cwasm")
        );
        assert!(instance.artifact.compatible_with(&runtime));
        assert_eq!(
            instance.store.limits.as_ref().and_then(|l| l.memory_bytes),
            Some(64 * 1024 * 1024)
        );
        assert_eq!(
            instance.linker.callbacks.as_ref().map(|c| c.imports.len()),
            Some(1)
        );
    }

    #[test]
    fn instance_and_memory_operations_fail_honestly_until_backend_lands() {
        let runtime = RuntimeSpec::new().build();
        let artifact = runtime.compile("add.wasm", SourceKind::Module);
        let store = runtime.store(None, None, None);
        let linker = runtime.linker();
        let instance = artifact.instantiate(store, linker).unwrap();

        let call_err = instance
            .call("add", vec![Value::I32(1), Value::I32(2)])
            .unwrap_err();
        assert_eq!(call_err.kind, RwasmtimeErrorKind::NotImplemented);
        assert!(call_err.message.contains("wt_call(add)"));

        let memory = instance.memory("memory");
        let span = MemorySpan::new(1024, 16, MemoryDType::F64)
            .dim(vec![2])
            .layout(MemoryLayout::ColumnMajor);
        assert_eq!(span.expected_len_bytes(), Some(16));
        let read_err = memory.read(&span).unwrap_err();
        assert_eq!(read_err.kind, RwasmtimeErrorKind::NotImplemented);
        assert!(read_err.message.contains("wt_memory_read"));
    }

    #[test]
    fn memory_span_validates_shape_bounds_and_write_payload() {
        let span = MemorySpan::new(8, 16, MemoryDType::F64).dim(vec![2]);
        assert_eq!(span.end().unwrap(), 24);
        assert_eq!(span.element_count(), Some(2));
        assert_eq!(span.expected_len_bytes(), Some(16));
        assert!(span.validate_bounds(24).is_ok());

        let err = span.validate_bounds(23).unwrap_err();
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("exceeds memory size"));

        let bad_shape = MemorySpan::new(0, 10, MemoryDType::F64).dim(vec![2]);
        let err = bad_shape.validate_shape().unwrap_err();
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("not a multiple"));

        let memory = Memory {
            name: "memory".to_string(),
        };
        let err = memory.write(&span, &[0; 8]).unwrap_err();
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("payload"));
    }

    #[test]
    fn incompatible_runtime_objects_do_not_instantiate() {
        let runtime_a = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift())
            .features(FeatureSpec::new().simd(true))
            .build();
        let runtime_b = RuntimeSpec::new()
            .compiler(CompilerSpec::winch())
            .features(FeatureSpec::new().simd(false))
            .build();

        let artifact = runtime_a.compile("tool.wasm", SourceKind::Module);
        let store = runtime_b.store(None, None, None);
        let linker = runtime_a.linker();
        let err = artifact.instantiate(store, linker).unwrap_err();
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
    }

    #[test]
    fn incompatible_aot_metadata_fails_before_instantiation() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift())
            .features(FeatureSpec::new().component_model(true).simd(true))
            .build();
        let mut artifact = runtime.compile("tool.wasm", SourceKind::Module);
        artifact.metadata = artifact.metadata.clone().with_format_version(999);
        let store = runtime.store(None, None, None);
        let linker = runtime.linker();

        let err = artifact.instantiate(store, linker).unwrap_err();
        assert_eq!(err.kind, RwasmtimeErrorKind::AotIncompatible);
        assert!(err.message.contains("format_version"));
    }
}
