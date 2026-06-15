use crate::app::{Result, RwasmtimeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    pub memory_bytes: Option<u64>,
    pub table_elements: Option<u64>,
    pub instances: Option<u64>,
    pub fuel: Option<u64>,
    pub wall_time_ms: Option<u64>,
    pub max_callback_calls: Option<u64>,
    pub callback_timeout_ms: Option<u64>,
    pub callback_max_depth: u32,
    pub callback_reentrant: bool,
}

impl Limits {
    pub fn new() -> Self {
        Self {
            memory_bytes: None,
            table_elements: None,
            instances: None,
            fuel: None,
            wall_time_ms: None,
            max_callback_calls: None,
            callback_timeout_ms: None,
            callback_max_depth: 1,
            callback_reentrant: false,
        }
    }

    pub fn memory_bytes(mut self, value: u64) -> Self {
        self.memory_bytes = Some(value);
        self
    }
    pub fn table_elements(mut self, value: u64) -> Self {
        self.table_elements = Some(value);
        self
    }
    pub fn instances(mut self, value: u64) -> Self {
        self.instances = Some(value);
        self
    }
    pub fn fuel(mut self, value: u64) -> Self {
        self.fuel = Some(value);
        self
    }
    pub fn wall_time_ms(mut self, value: u64) -> Self {
        self.wall_time_ms = Some(value);
        self
    }
    pub fn max_callback_calls(mut self, value: u64) -> Self {
        self.max_callback_calls = Some(value);
        self
    }
    pub fn callback_timeout_ms(mut self, value: u64) -> Self {
        self.callback_timeout_ms = Some(value);
        self
    }

    pub fn callback_depth(mut self, max_depth: u32, reentrant: bool) -> Self {
        self.callback_max_depth = max_depth;
        self.callback_reentrant = reentrant;
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.callback_max_depth == 0 {
            return Err(RwasmtimeError::invalid_argument(
                "callback max depth must be at least 1",
            ));
        }
        if self.callback_reentrant && self.callback_max_depth < 2 {
            return Err(RwasmtimeError::invalid_argument(
                "reentrant callbacks require callback max depth of at least 2",
            ));
        }
        Ok(())
    }

    pub fn has_resource_caps(&self) -> bool {
        self.memory_bytes.is_some()
            || self.table_elements.is_some()
            || self.instances.is_some()
            || self.fuel.is_some()
            || self.wall_time_ms.is_some()
    }

    pub fn callback_calls_allowed(&self) -> Option<u64> {
        self.max_callback_calls
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_are_unset_by_default_except_callback_safety() {
        let limits = Limits::new();
        assert_eq!(limits.memory_bytes, None);
        assert_eq!(limits.wall_time_ms, None);
        assert_eq!(limits.callback_max_depth, 1);
        assert!(!limits.callback_reentrant);
        assert!(!limits.has_resource_caps());
        assert_eq!(limits.callback_calls_allowed(), None);
        assert!(limits.validate().is_ok());
    }

    #[test]
    fn limits_builder_records_resource_caps() {
        let limits = Limits::new()
            .memory_bytes(512 * 1024 * 1024)
            .table_elements(20_000)
            .instances(32)
            .fuel(100_000_000)
            .wall_time_ms(5_000)
            .max_callback_calls(10_000)
            .callback_timeout_ms(1_000);

        assert_eq!(limits.memory_bytes, Some(512 * 1024 * 1024));
        assert_eq!(limits.instances, Some(32));
        assert_eq!(limits.max_callback_calls, Some(10_000));
        assert!(limits.has_resource_caps());
        assert_eq!(limits.callback_calls_allowed(), Some(10_000));
        assert!(limits.validate().is_ok());
    }

    #[test]
    fn zero_caps_are_explicit_not_missing() {
        let limits = Limits::new()
            .memory_bytes(0)
            .table_elements(0)
            .instances(0)
            .fuel(0)
            .wall_time_ms(0)
            .max_callback_calls(0)
            .callback_timeout_ms(0);

        assert_eq!(limits.memory_bytes, Some(0));
        assert_eq!(limits.max_callback_calls, Some(0));
        assert_ne!(limits.max_callback_calls, None);
        assert!(limits.has_resource_caps());
        assert!(limits.validate().is_ok());
    }

    #[test]
    fn callback_depth_validation_rejects_ambiguous_reentrancy() {
        let err = Limits::new()
            .callback_depth(0, false)
            .validate()
            .expect_err("depth zero is ambiguous");
        assert!(err.message.contains("at least 1"));

        let err = Limits::new()
            .callback_depth(1, true)
            .validate()
            .expect_err("reentrant depth one is ambiguous");
        assert!(err.message.contains("at least 2"));
    }
}
