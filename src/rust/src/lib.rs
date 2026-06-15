//! Rwasmtime Rust core scaffold.
//!
//! The real backend will own Wasmtime `Engine`, `Store`, `Linker`, module,
//! component, memory, AOT, WASI, and async job state. This scaffold keeps the
//! public boundary explicit and R-free.

pub mod aot;
pub mod app;
pub mod arrays;
#[cfg(feature = "wasmtime")]
pub mod backend;
pub mod callbacks;
pub mod component;
pub mod config;
#[cfg(feature = "c-api")]
pub mod ffi;
pub mod limits;
pub mod repl;
pub mod runtime_objects;
pub mod wasi;

pub use aot::{ArtifactCompatibility, ArtifactCompatibilityIssue, ArtifactMetadata, ARTIFACT_FORMAT_VERSION};
pub use app::{AppSpec, ArrayPolicy, Job, JobPoll, JobState, PreparedApp, RwasmtimeError, RwasmtimeErrorKind, Session, SourceKind, Value};
pub use arrays::{ArrayAllocator, ArrayArgument, ArrayBuffer, ArrayWriteRequest, BufferLayout, MemoryView, MemoryViewLifetime};
#[cfg(feature = "wasmtime")]
pub use backend::WasmtimeRuntime;
pub use callbacks::{CallbackAbi, CallbackBrokerQueue, CallbackCompletion, CallbackError, CallbackPolicy, CallbackReply, CallbackRequest, CallbackReturnPath, CallbackServicePlan, CallbackSet, CallbackSpec, CallbackTicket, CallbackWakeStrategy, HostCallbackBroker, PendingCallback};
pub use component::{ComponentCallRequest, ComponentItem, ComponentItemKind, ComponentSpec, WitCase, WitField, WitType, WitValue, WitValueMismatch};
pub use config::{AllocatorSpec, AotSpec, CompilerSpec, CompilerStrategy, FeatureSpec, RuntimeSpec};
pub use limits::Limits;
pub use repl::{ReplProtocol, ReplRequest, ReplResult, ReplSession, ReplSpec};
pub use runtime_objects::{Artifact, ArtifactInfo, Instance, Linker, Memory, MemoryDType, MemoryLayout, MemorySpan, Store};
pub use wasi::{StdioMode, WasiPreopen, WasiSpec};

/// Runtime handle placeholder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runtime {
    pub spec: RuntimeSpec,
}

impl RuntimeSpec {
    pub fn build(self) -> Runtime {
        Runtime { spec: self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_spec_builder_keeps_compiler_and_features() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(true).simd(true))
            .build();

        assert_eq!(runtime.spec.compiler.strategy, CompilerStrategy::Cranelift);
        assert!(runtime.spec.features.component_model);
        assert!(runtime.spec.features.simd);
    }

}
