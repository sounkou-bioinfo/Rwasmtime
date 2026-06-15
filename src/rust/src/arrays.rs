use crate::app::{ArrayDType, Result, RwasmtimeError, Session};
use crate::runtime_objects::{Memory, MemoryDType};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferLayout {
    ColumnMajor,
    RowMajor,
    Contiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayAllocator {
    Guest,
    HostArena,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayWriteRequest {
    pub bytes: Vec<u8>,
    pub dtype: ArrayDType,
    pub dim: Option<Vec<u64>>,
    pub layout: BufferLayout,
    pub allocator: ArrayAllocator,
    pub alloc_export: String,
    pub free_export: String,
}

impl ArrayDType {
    pub fn byte_width(self) -> u64 {
        match self {
            ArrayDType::U8 => 1,
            ArrayDType::I32 | ArrayDType::F32 => 4,
            ArrayDType::I64 | ArrayDType::F64 => 8,
        }
    }
}

impl ArrayWriteRequest {
    pub fn new(bytes: Vec<u8>, dtype: ArrayDType) -> Self {
        Self {
            bytes,
            dtype,
            dim: None,
            layout: BufferLayout::ColumnMajor,
            allocator: ArrayAllocator::Guest,
            alloc_export: "alloc".to_string(),
            free_export: "free".to_string(),
        }
    }

    pub fn dim(mut self, value: Vec<u64>) -> Self {
        self.dim = Some(value);
        self
    }

    pub fn layout(mut self, value: BufferLayout) -> Self {
        self.layout = value;
        self
    }

    pub fn allocator(mut self, value: ArrayAllocator) -> Self {
        self.allocator = value;
        self
    }

    pub fn alloc_export(mut self, value: impl Into<String>) -> Self {
        self.alloc_export = value.into();
        self
    }

    pub fn free_export(mut self, value: impl Into<String>) -> Self {
        self.free_export = value.into();
        self
    }

    pub fn element_count(&self) -> Option<u64> {
        product_dims(self.dim.as_deref())
    }

    pub fn expected_len_bytes(&self) -> Option<u64> {
        self.element_count()?.checked_mul(self.dtype.byte_width())
    }

    pub fn validate(&self) -> Result<()> {
        if self.alloc_export.is_empty() {
            return Err(RwasmtimeError::invalid_argument(
                "array allocation export must not be empty",
            ));
        }
        if self.free_export.is_empty() {
            return Err(RwasmtimeError::invalid_argument(
                "array free export must not be empty",
            ));
        }
        let width = self.dtype.byte_width();
        if (self.bytes.len() as u64) % width != 0 {
            return Err(RwasmtimeError::invalid_argument(format!(
                "array byte length {} is not a multiple of {} for {:?}",
                self.bytes.len(),
                width,
                self.dtype
            )));
        }
        if let Some(expected) = self.expected_len_bytes() {
            if expected != self.bytes.len() as u64 {
                return Err(RwasmtimeError::invalid_argument(format!(
                    "array dim implies {expected} bytes but payload has {} bytes",
                    self.bytes.len()
                )));
            }
        }
        Ok(())
    }
}

fn product_dims(dim: Option<&[u64]>) -> Option<u64> {
    let dim = dim?;
    dim.iter()
        .try_fold(1_u64, |acc, value| acc.checked_mul(*value))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayBuffer {
    pub ptr: u64,
    pub len_bytes: u64,
    pub dtype: ArrayDType,
    pub dim: Option<Vec<u64>>,
    pub layout: BufferLayout,
    pub allocator: ArrayAllocator,
    pub free_export: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArrayArgument {
    pub name: String,
}

impl ArrayArgument {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryViewLifetime {
    UntilNextWasmCall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryView {
    pub memory_name: String,
    pub ptr: u64,
    pub len: u64,
    pub dtype: MemoryDType,
    pub mutable: bool,
    pub lifetime: MemoryViewLifetime,
}

impl Memory {
    pub fn view(
        &self,
        ptr: u64,
        len: u64,
        dtype: MemoryDType,
        mutable: bool,
        lifetime: MemoryViewLifetime,
    ) -> MemoryView {
        MemoryView {
            memory_name: self.name.clone(),
            ptr,
            len,
            dtype,
            mutable,
            lifetime,
        }
    }
}

impl Session {
    pub fn array_write(&self, request: ArrayWriteRequest) -> Result<ArrayBuffer> {
        request.validate()?;
        Err(RwasmtimeError::not_implemented("wt_array_write"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppSpec, RwasmtimeErrorKind};
    use crate::runtime_objects::MemoryLayout;

    #[test]
    fn array_write_request_records_explicit_transport_policy() {
        let request = ArrayWriteRequest::new(vec![0; 16], ArrayDType::F64)
            .dim(vec![2, 1])
            .layout(BufferLayout::ColumnMajor)
            .allocator(ArrayAllocator::Guest)
            .alloc_export("guest_alloc")
            .free_export("guest_free");

        assert_eq!(request.dtype, ArrayDType::F64);
        assert_eq!(request.dim.as_deref(), Some(&[2, 1][..]));
        assert_eq!(request.layout, BufferLayout::ColumnMajor);
        assert_eq!(request.allocator, ArrayAllocator::Guest);
        assert_eq!(request.alloc_export, "guest_alloc");
        assert_eq!(request.free_export, "guest_free");
        assert_eq!(request.element_count(), Some(2));
        assert_eq!(request.expected_len_bytes(), Some(16));
        assert!(request.validate().is_ok());
    }

    #[test]
    fn array_write_request_rejects_mismatched_shape_and_payload() {
        let err = ArrayWriteRequest::new(vec![0; 15], ArrayDType::F64)
            .dim(vec![2])
            .validate()
            .expect_err("f64 payload must be aligned and match dims");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("not a multiple"));

        let err = ArrayWriteRequest::new(vec![0; 16], ArrayDType::F64)
            .dim(vec![3])
            .validate()
            .expect_err("dim mismatch should be reported before backend transport");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("dim implies"));
    }

    #[test]
    fn session_array_write_fails_honestly_until_memory_transport_lands() {
        let session = AppSpec::new("stats_plugin.component.wasm")
            .component()
            .prepare()
            .new_session();
        let err = session
            .array_write(ArrayWriteRequest::new(vec![0; 8], ArrayDType::F64))
            .expect_err("array transport is backend work");

        assert_eq!(err.kind, RwasmtimeErrorKind::NotImplemented);
        assert!(err.message.contains("wt_array_write"));
    }

    #[test]
    fn borrowed_memory_view_carries_visible_lifetime() {
        let memory = Memory {
            name: "memory".to_string(),
        };
        let view = memory.view(
            1024,
            1000,
            MemoryDType::F64,
            false,
            MemoryViewLifetime::UntilNextWasmCall,
        );

        assert_eq!(view.memory_name, "memory");
        assert_eq!(view.ptr, 1024);
        assert_eq!(view.len, 1000);
        assert_eq!(view.dtype, MemoryDType::F64);
        assert!(!view.mutable);
        assert_eq!(view.lifetime, MemoryViewLifetime::UntilNextWasmCall);

        let span = crate::runtime_objects::MemorySpan::new(1024, 1000, MemoryDType::F64)
            .layout(MemoryLayout::ColumnMajor);
        assert_eq!(span.layout, MemoryLayout::ColumnMajor);
    }

    #[test]
    fn array_argument_is_named_reference_not_payload() {
        let arg = ArrayArgument::new("x");
        assert_eq!(arg.name, "x");
    }
}
