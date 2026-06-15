use crate::app::{Result, RwasmtimeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerStrategy {
    Auto,
    Cranelift,
    Winch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    None,
    Speed,
    SpeedAndSize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompilerSpec {
    pub strategy: CompilerStrategy,
    pub opt_level: OptLevel,
    pub parallel: bool,
}

impl CompilerSpec {
    pub fn auto() -> Self {
        Self {
            strategy: CompilerStrategy::Auto,
            opt_level: OptLevel::Speed,
            parallel: true,
        }
    }

    pub fn cranelift() -> Self {
        Self {
            strategy: CompilerStrategy::Cranelift,
            opt_level: OptLevel::Speed,
            parallel: true,
        }
    }

    pub fn winch() -> Self {
        Self {
            strategy: CompilerStrategy::Winch,
            opt_level: OptLevel::None,
            parallel: true,
        }
    }

    pub fn none(mut self) -> Self {
        self.opt_level = OptLevel::None;
        self
    }

    pub fn speed(mut self) -> Self {
        self.opt_level = OptLevel::Speed;
        self
    }

    pub fn speed_and_size(mut self) -> Self {
        self.opt_level = OptLevel::SpeedAndSize;
        self
    }

    pub fn parallel(mut self, value: bool) -> Self {
        self.parallel = value;
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.strategy == CompilerStrategy::Winch && self.opt_level != OptLevel::None {
            return Err(RwasmtimeError::invalid_argument(
                "winch compiler requires opt_level none",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureSpec {
    pub component_model: bool,
    pub component_model_async: bool,
    pub simd: bool,
    pub relaxed_simd: bool,
    pub relaxed_simd_deterministic: bool,
    pub bulk_memory: bool,
    pub multi_memory: bool,
    pub memory64: bool,
    pub threads: bool,
    pub exceptions: bool,
    pub legacy_exceptions: bool,
    pub gc: bool,
}

impl FeatureSpec {
    pub fn new() -> Self {
        Self {
            component_model: true,
            component_model_async: false,
            simd: true,
            relaxed_simd: false,
            relaxed_simd_deterministic: false,
            bulk_memory: true,
            multi_memory: true,
            memory64: false,
            threads: false,
            exceptions: false,
            legacy_exceptions: false,
            gc: false,
        }
    }

    pub fn component_model(mut self, value: bool) -> Self {
        self.component_model = value;
        self
    }
    pub fn component_model_async(mut self, value: bool) -> Self {
        self.component_model_async = value;
        self
    }
    pub fn simd(mut self, value: bool) -> Self {
        self.simd = value;
        self
    }
    pub fn relaxed_simd(mut self, value: bool) -> Self {
        self.relaxed_simd = value;
        self
    }
    pub fn relaxed_simd_deterministic(mut self, value: bool) -> Self {
        self.relaxed_simd_deterministic = value;
        self
    }
    pub fn memory64(mut self, value: bool) -> Self {
        self.memory64 = value;
        self
    }
    pub fn threads(mut self, value: bool) -> Self {
        self.threads = value;
        self
    }
    pub fn exceptions(mut self, value: bool) -> Self {
        self.exceptions = value;
        self
    }
    pub fn legacy_exceptions(mut self, value: bool) -> Self {
        self.legacy_exceptions = value;
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.component_model_async && !self.component_model {
            return Err(RwasmtimeError::invalid_argument(
                "component_model_async requires component_model",
            ));
        }
        if self.relaxed_simd && !self.simd {
            return Err(RwasmtimeError::invalid_argument(
                "relaxed_simd requires simd",
            ));
        }
        if self.relaxed_simd_deterministic && !self.relaxed_simd {
            return Err(RwasmtimeError::invalid_argument(
                "relaxed_simd_deterministic requires relaxed_simd",
            ));
        }
        if self.legacy_exceptions && !self.exceptions {
            return Err(RwasmtimeError::invalid_argument(
                "legacy_exceptions requires exceptions",
            ));
        }
        Ok(())
    }
}

impl Default for FeatureSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AotSpec {
    pub cache: bool,
    pub cache_dir: Option<String>,
    pub artifact_dir: Option<String>,
}

impl AotSpec {
    pub fn new() -> Self {
        Self {
            cache: true,
            cache_dir: None,
            artifact_dir: None,
        }
    }

    pub fn cache(mut self, value: bool) -> Self {
        self.cache = value;
        self
    }
    pub fn cache_dir(mut self, value: impl Into<String>) -> Self {
        self.cache_dir = Some(value.into());
        self
    }
    pub fn artifact_dir(mut self, value: impl Into<String>) -> Self {
        self.artifact_dir = Some(value.into());
        self
    }
}

impl Default for AotSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocatorStrategy {
    OnDemand,
    Pooling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AllocatorSpec {
    pub strategy: AllocatorStrategy,
    pub memory_limit: Option<u64>,
    pub table_limit: Option<u64>,
    pub instance_limit: Option<u64>,
}

impl AllocatorSpec {
    pub fn on_demand() -> Self {
        Self {
            strategy: AllocatorStrategy::OnDemand,
            memory_limit: None,
            table_limit: None,
            instance_limit: None,
        }
    }

    pub fn pooling() -> Self {
        Self {
            strategy: AllocatorStrategy::Pooling,
            memory_limit: None,
            table_limit: None,
            instance_limit: None,
        }
    }

    pub fn memory_limit(mut self, bytes: u64) -> Self {
        self.memory_limit = Some(bytes);
        self
    }
    pub fn table_limit(mut self, elements: u64) -> Self {
        self.table_limit = Some(elements);
        self
    }
    pub fn instance_limit(mut self, n: u64) -> Self {
        self.instance_limit = Some(n);
        self
    }
}

impl Default for AllocatorSpec {
    fn default() -> Self {
        Self::on_demand()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSpec {
    pub compiler: CompilerSpec,
    pub features: FeatureSpec,
    pub aot: AotSpec,
    pub allocator: AllocatorSpec,
}

impl RuntimeSpec {
    pub fn new() -> Self {
        Self {
            compiler: CompilerSpec::auto(),
            features: FeatureSpec::default(),
            aot: AotSpec::default(),
            allocator: AllocatorSpec::default(),
        }
    }

    pub fn compiler(mut self, compiler: CompilerSpec) -> Self {
        self.compiler = compiler;
        self
    }
    pub fn features(mut self, features: FeatureSpec) -> Self {
        self.features = features;
        self
    }
    pub fn aot(mut self, aot: AotSpec) -> Self {
        self.aot = aot;
        self
    }
    pub fn allocator(mut self, allocator: AllocatorSpec) -> Self {
        self.allocator = allocator;
        self
    }

    pub fn validate(&self) -> Result<()> {
        self.compiler.validate()?;
        self.features.validate()?;
        Ok(())
    }
}

impl Default for RuntimeSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::RwasmtimeErrorKind;

    #[test]
    fn relaxed_simd_determinism_is_off_by_default() {
        let features = FeatureSpec::new();
        assert!(features.simd);
        assert!(!features.relaxed_simd);
        assert!(!features.relaxed_simd_deterministic);
        assert!(!features.exceptions);
        assert!(!features.legacy_exceptions);
        assert!(features.validate().is_ok());
    }

    #[test]
    fn feature_validation_requires_explicit_relaxed_simd_for_determinism() {
        let err = FeatureSpec::new()
            .relaxed_simd_deterministic(true)
            .validate()
            .expect_err("deterministic relaxed SIMD requires relaxed SIMD");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("relaxed_simd_deterministic"));

        let err = FeatureSpec::new()
            .simd(false)
            .relaxed_simd(true)
            .validate()
            .expect_err("relaxed SIMD requires SIMD");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("relaxed_simd requires simd"));

        assert!(FeatureSpec::new()
            .relaxed_simd(true)
            .relaxed_simd_deterministic(true)
            .validate()
            .is_ok());
    }

    #[test]
    fn feature_validation_requires_exceptions_for_legacy_exceptions() {
        let err = FeatureSpec::new()
            .legacy_exceptions(true)
            .validate()
            .expect_err("legacy exceptions require exceptions");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("legacy_exceptions"));

        assert!(FeatureSpec::new()
            .exceptions(true)
            .legacy_exceptions(true)
            .validate()
            .is_ok());
    }

    #[test]
    fn runtime_validation_checks_compiler_and_feature_consistency() {
        let err = RuntimeSpec::new()
            .compiler(CompilerSpec::winch().speed())
            .validate()
            .expect_err("winch does not accept optimizing opt levels in scaffold");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("winch"));

        let err = RuntimeSpec::new()
            .features(
                FeatureSpec::new()
                    .component_model(false)
                    .component_model_async(true),
            )
            .validate()
            .expect_err("async component model requires component model");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("component_model_async"));
    }
}
