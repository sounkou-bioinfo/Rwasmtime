use crate::app::{Result, RwasmtimeError, RwasmtimeErrorKind, SourceKind};
use crate::config::{CompilerStrategy, FeatureSpec, OptLevel, RuntimeSpec};
use crate::Runtime;

pub const ARTIFACT_FORMAT_VERSION: u32 = 1;
pub const SCAFFOLD_WASMTIME_VERSION: &str = "scaffold";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactMetadata {
    pub format_version: u32,
    pub rwasmtime_version: String,
    pub wasmtime_version: String,
    pub target: String,
    pub kind: SourceKind,
    pub compiler: CompilerStrategy,
    pub opt_level: OptLevel,
    pub compiler_parallel: bool,
    pub features: FeatureSpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactCompatibilityIssue {
    FormatVersion { expected: u32, found: u32 },
    RwasmtimeVersion { expected: String, found: String },
    WasmtimeVersion { expected: String, found: String },
    Target { expected: String, found: String },
    SourceKind { expected: SourceKind, found: SourceKind },
    CompilerStrategy { expected: CompilerStrategy, found: CompilerStrategy },
    OptLevel { expected: OptLevel, found: OptLevel },
    Feature { name: &'static str, expected: bool, found: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactCompatibility {
    pub issues: Vec<ArtifactCompatibilityIssue>,
}

impl ArtifactCompatibility {
    pub fn compatible() -> Self {
        Self { issues: Vec::new() }
    }

    pub fn is_compatible(&self) -> bool {
        self.issues.is_empty()
    }

    pub fn into_result(self) -> Result<()> {
        if self.is_compatible() {
            Ok(())
        } else {
            Err(RwasmtimeError {
                kind: RwasmtimeErrorKind::AotIncompatible,
                message: format!("AOT artifact is incompatible: {}", self.summary()),
            })
        }
    }

    pub fn summary(&self) -> String {
        if self.issues.is_empty() {
            return "compatible".to_string();
        }
        self.issues
            .iter()
            .map(|issue| match issue {
                ArtifactCompatibilityIssue::FormatVersion { expected, found } => {
                    format!("format_version expected {expected}, found {found}")
                }
                ArtifactCompatibilityIssue::RwasmtimeVersion { expected, found } => {
                    format!("rwasmtime_version expected {expected}, found {found}")
                }
                ArtifactCompatibilityIssue::WasmtimeVersion { expected, found } => {
                    format!("wasmtime_version expected {expected}, found {found}")
                }
                ArtifactCompatibilityIssue::Target { expected, found } => {
                    format!("target expected {expected}, found {found}")
                }
                ArtifactCompatibilityIssue::SourceKind { expected, found } => {
                    format!("source kind expected {expected:?}, found {found:?}")
                }
                ArtifactCompatibilityIssue::CompilerStrategy { expected, found } => {
                    format!("compiler strategy expected {expected:?}, found {found:?}")
                }
                ArtifactCompatibilityIssue::OptLevel { expected, found } => {
                    format!("optimization level expected {expected:?}, found {found:?}")
                }
                ArtifactCompatibilityIssue::Feature { name, expected, found } => {
                    format!("feature {name} expected {expected}, found {found}")
                }
            })
            .collect::<Vec<_>>()
            .join("; ")
    }
}

impl ArtifactMetadata {
    pub fn from_runtime(runtime: &Runtime, kind: SourceKind) -> Self {
        Self::from_runtime_spec(&runtime.spec, kind)
    }

    pub fn from_runtime_spec(spec: &RuntimeSpec, kind: SourceKind) -> Self {
        Self {
            format_version: ARTIFACT_FORMAT_VERSION,
            rwasmtime_version: env!("CARGO_PKG_VERSION").to_string(),
            wasmtime_version: SCAFFOLD_WASMTIME_VERSION.to_string(),
            target: current_target(),
            kind,
            compiler: spec.compiler.strategy,
            opt_level: spec.compiler.opt_level,
            compiler_parallel: spec.compiler.parallel,
            features: spec.features,
        }
    }

    pub fn with_format_version(mut self, value: u32) -> Self {
        self.format_version = value;
        self
    }

    pub fn with_rwasmtime_version(mut self, value: impl Into<String>) -> Self {
        self.rwasmtime_version = value.into();
        self
    }

    pub fn with_wasmtime_version(mut self, value: impl Into<String>) -> Self {
        self.wasmtime_version = value.into();
        self
    }

    pub fn with_target(mut self, value: impl Into<String>) -> Self {
        self.target = value.into();
        self
    }

    pub fn with_kind(mut self, value: SourceKind) -> Self {
        self.kind = value;
        self
    }

    pub fn with_features(mut self, value: FeatureSpec) -> Self {
        self.features = value;
        self
    }

    pub fn compatibility_with(&self, runtime: &Runtime, expected_kind: SourceKind) -> ArtifactCompatibility {
        self.compatibility_with_spec(&runtime.spec, expected_kind)
    }

    pub fn compatibility_with_spec(&self, spec: &RuntimeSpec, expected_kind: SourceKind) -> ArtifactCompatibility {
        let mut issues = Vec::new();
        push_ne_u32(
            &mut issues,
            ARTIFACT_FORMAT_VERSION,
            self.format_version,
            |expected, found| ArtifactCompatibilityIssue::FormatVersion { expected, found },
        );
        push_ne_string(
            &mut issues,
            env!("CARGO_PKG_VERSION"),
            &self.rwasmtime_version,
            |expected, found| ArtifactCompatibilityIssue::RwasmtimeVersion { expected, found },
        );
        push_ne_string(
            &mut issues,
            SCAFFOLD_WASMTIME_VERSION,
            &self.wasmtime_version,
            |expected, found| ArtifactCompatibilityIssue::WasmtimeVersion { expected, found },
        );
        push_ne_string(
            &mut issues,
            &current_target(),
            &self.target,
            |expected, found| ArtifactCompatibilityIssue::Target { expected, found },
        );
        if self.kind != expected_kind {
            issues.push(ArtifactCompatibilityIssue::SourceKind { expected: expected_kind, found: self.kind });
        }
        if self.compiler != spec.compiler.strategy {
            issues.push(ArtifactCompatibilityIssue::CompilerStrategy { expected: spec.compiler.strategy, found: self.compiler });
        }
        if self.opt_level != spec.compiler.opt_level {
            issues.push(ArtifactCompatibilityIssue::OptLevel { expected: spec.compiler.opt_level, found: self.opt_level });
        }
        compare_features(&mut issues, &spec.features, &self.features);
        ArtifactCompatibility { issues }
    }

    pub fn compatible_with(&self, runtime: &Runtime, expected_kind: SourceKind) -> bool {
        self.compatibility_with(runtime, expected_kind).is_compatible()
    }

    pub fn ensure_compatible_with_spec(&self, spec: &RuntimeSpec, expected_kind: SourceKind) -> Result<()> {
        self.compatibility_with_spec(spec, expected_kind).into_result()
    }
}

pub fn current_target() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

fn push_ne_u32<F>(issues: &mut Vec<ArtifactCompatibilityIssue>, expected: u32, found: u32, make: F)
where
    F: FnOnce(u32, u32) -> ArtifactCompatibilityIssue,
{
    if expected != found {
        issues.push(make(expected, found));
    }
}

fn push_ne_string<F>(issues: &mut Vec<ArtifactCompatibilityIssue>, expected: &str, found: &str, make: F)
where
    F: FnOnce(String, String) -> ArtifactCompatibilityIssue,
{
    if expected != found {
        issues.push(make(expected.to_string(), found.to_string()));
    }
}

fn compare_features(issues: &mut Vec<ArtifactCompatibilityIssue>, expected: &FeatureSpec, found: &FeatureSpec) {
    feature(issues, "component_model", expected.component_model, found.component_model);
    feature(issues, "component_model_async", expected.component_model_async, found.component_model_async);
    feature(issues, "simd", expected.simd, found.simd);
    feature(issues, "relaxed_simd", expected.relaxed_simd, found.relaxed_simd);
    feature(issues, "relaxed_simd_deterministic", expected.relaxed_simd_deterministic, found.relaxed_simd_deterministic);
    feature(issues, "bulk_memory", expected.bulk_memory, found.bulk_memory);
    feature(issues, "multi_memory", expected.multi_memory, found.multi_memory);
    feature(issues, "memory64", expected.memory64, found.memory64);
    feature(issues, "threads", expected.threads, found.threads);
    feature(issues, "gc", expected.gc, found.gc);
}

fn feature(issues: &mut Vec<ArtifactCompatibilityIssue>, name: &'static str, expected: bool, found: bool) {
    if expected != found {
        issues.push(ArtifactCompatibilityIssue::Feature { name, expected, found });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CompilerSpec, FeatureSpec, RuntimeSpec};

    #[test]
    fn artifact_metadata_records_compiler_features_target_and_versions() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed_and_size().parallel(false))
            .features(FeatureSpec::new().component_model(true).simd(true).memory64(false))
            .build();

        let metadata = ArtifactMetadata::from_runtime(&runtime, SourceKind::Component);
        assert_eq!(metadata.format_version, ARTIFACT_FORMAT_VERSION);
        assert_eq!(metadata.rwasmtime_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(metadata.wasmtime_version, SCAFFOLD_WASMTIME_VERSION);
        assert_eq!(metadata.target, current_target());
        assert_eq!(metadata.kind, SourceKind::Component);
        assert_eq!(metadata.compiler, CompilerStrategy::Cranelift);
        assert_eq!(metadata.opt_level, crate::config::OptLevel::SpeedAndSize);
        assert!(!metadata.compiler_parallel);
        assert!(metadata.compatible_with(&runtime, SourceKind::Component));
    }

    #[test]
    fn artifact_compatibility_reports_all_incompatible_metadata_before_execution() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift().speed())
            .features(FeatureSpec::new().component_model(true).simd(true))
            .build();
        let metadata = ArtifactMetadata::from_runtime(&runtime, SourceKind::Module)
            .with_format_version(999)
            .with_rwasmtime_version("other-rwasmtime")
            .with_wasmtime_version("other-wasmtime")
            .with_target("other-target")
            .with_kind(SourceKind::Component)
            .with_features(FeatureSpec::new().component_model(false).simd(false));

        let report = metadata.compatibility_with(&runtime, SourceKind::Module);
        assert!(!report.is_compatible());
        assert!(report.issues.iter().any(|i| matches!(i, ArtifactCompatibilityIssue::FormatVersion { .. })));
        assert!(report.issues.iter().any(|i| matches!(i, ArtifactCompatibilityIssue::RwasmtimeVersion { .. })));
        assert!(report.issues.iter().any(|i| matches!(i, ArtifactCompatibilityIssue::WasmtimeVersion { .. })));
        assert!(report.issues.iter().any(|i| matches!(i, ArtifactCompatibilityIssue::Target { .. })));
        assert!(report.issues.iter().any(|i| matches!(i, ArtifactCompatibilityIssue::SourceKind { .. })));
        assert!(report.issues.iter().any(|i| matches!(i, ArtifactCompatibilityIssue::Feature { name: "component_model", .. })));
        assert!(report.issues.iter().any(|i| matches!(i, ArtifactCompatibilityIssue::Feature { name: "simd", .. })));
        let err = report.into_result().expect_err("incompatible artifact should fail");
        assert_eq!(err.kind, RwasmtimeErrorKind::AotIncompatible);
        assert!(err.message.contains("format_version"));
    }
}
