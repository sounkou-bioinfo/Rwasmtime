use std::collections::BTreeMap;

use crate::app::{Result, RwasmtimeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StdioMode {
    Empty,
    Inherit,
    String,
    File,
    Capture,
    Discard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasiPreopen {
    pub guest: String,
    pub host: String,
    pub readonly: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasiSpec {
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub preopens: Vec<WasiPreopen>,
    pub stdin: StdioMode,
    pub stdout: StdioMode,
    pub stderr: StdioMode,
    pub stdin_bytes: Option<Vec<u8>>,
    pub stdout_file: Option<String>,
    pub stderr_file: Option<String>,
    pub network: bool,
    pub clocks: bool,
    pub random: bool,
}

impl WasiPreopen {
    pub fn validate(&self) -> Result<()> {
        if self.guest.is_empty() || !self.guest.starts_with('/') {
            return Err(RwasmtimeError::invalid_argument("WASI preopen guest path must be a non-empty absolute path"));
        }
        if self.host.is_empty() {
            return Err(RwasmtimeError::invalid_argument("WASI preopen host path must be non-empty"));
        }
        if self.guest.contains('\0') || self.host.contains('\0') {
            return Err(RwasmtimeError::invalid_argument("WASI preopen paths must not contain NUL bytes"));
        }
        Ok(())
    }
}

impl WasiSpec {
    pub fn new() -> Self {
        Self {
            args: Vec::new(),
            env: BTreeMap::new(),
            preopens: Vec::new(),
            stdin: StdioMode::Empty,
            stdout: StdioMode::Capture,
            stderr: StdioMode::Capture,
            stdin_bytes: None,
            stdout_file: None,
            stderr_file: None,
            network: false,
            clocks: false,
            random: false,
        }
    }

    pub fn arg(mut self, value: impl Into<String>) -> Self {
        self.args.push(value.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    pub fn preopen(
        mut self,
        guest: impl Into<String>,
        host: impl Into<String>,
        readonly: bool,
    ) -> Self {
        self.preopens.push(WasiPreopen { guest: guest.into(), host: host.into(), readonly });
        self
    }

    pub fn stdio(mut self, stdin: StdioMode, stdout: StdioMode, stderr: StdioMode) -> Self {
        self.stdin = stdin;
        self.stdout = stdout;
        self.stderr = stderr;
        self
    }

    pub fn stdin_text(mut self, value: impl Into<String>) -> Self {
        self.stdin = StdioMode::String;
        self.stdin_bytes = Some(value.into().into_bytes());
        self
    }

    pub fn stdin_bytes(mut self, value: impl Into<Vec<u8>>) -> Self {
        self.stdin = StdioMode::String;
        self.stdin_bytes = Some(value.into());
        self
    }

    pub fn stdout_file(mut self, path: impl Into<String>) -> Self {
        self.stdout = StdioMode::File;
        self.stdout_file = Some(path.into());
        self
    }

    pub fn stderr_file(mut self, path: impl Into<String>) -> Self {
        self.stderr = StdioMode::File;
        self.stderr_file = Some(path.into());
        self
    }

    pub fn network(mut self, value: bool) -> Self { self.network = value; self }
    pub fn clocks(mut self, value: bool) -> Self { self.clocks = value; self }
    pub fn random(mut self, value: bool) -> Self { self.random = value; self }

    pub fn validate(&self) -> Result<()> {
        for arg in &self.args {
            if arg.contains('\0') {
                return Err(RwasmtimeError::invalid_argument("WASI args must not contain NUL bytes"));
            }
        }
        for (key, value) in &self.env {
            if key.is_empty() || key.contains('=') || key.contains('\0') {
                return Err(RwasmtimeError::invalid_argument("WASI env names must be non-empty and must not contain '=' or NUL bytes"));
            }
            if value.contains('\0') {
                return Err(RwasmtimeError::invalid_argument("WASI env values must not contain NUL bytes"));
            }
        }
        for preopen in &self.preopens {
            preopen.validate()?;
        }
        match self.stdin {
            StdioMode::String if self.stdin_bytes.is_none() => {
                return Err(RwasmtimeError::invalid_argument("WASI stdin string mode requires stdin bytes"));
            }
            StdioMode::File => {
                return Err(RwasmtimeError::invalid_argument("WASI stdin file mode is not represented in the scaffold yet"));
            }
            _ => {}
        }
        if self.stdout == StdioMode::File && self.stdout_file.as_deref().unwrap_or("").is_empty() {
            return Err(RwasmtimeError::invalid_argument("WASI stdout file mode requires stdout_file"));
        }
        if self.stderr == StdioMode::File && self.stderr_file.as_deref().unwrap_or("").is_empty() {
            return Err(RwasmtimeError::invalid_argument("WASI stderr file mode requires stderr_file"));
        }
        Ok(())
    }

    pub fn grants_filesystem(&self) -> bool {
        !self.preopens.is_empty()
    }

    pub fn grants_ambient_authority(&self) -> bool {
        self.network || self.clocks || self.random || self.stdin == StdioMode::Inherit || self.stdout == StdioMode::Inherit || self.stderr == StdioMode::Inherit
    }
}

impl Default for WasiSpec {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasi_is_deny_by_default() {
        let wasi = WasiSpec::new();
        assert!(wasi.args.is_empty());
        assert!(wasi.env.is_empty());
        assert!(wasi.preopens.is_empty());
        assert_eq!(wasi.stdin, StdioMode::Empty);
        assert_eq!(wasi.stdout, StdioMode::Capture);
        assert!(!wasi.network);
        assert!(!wasi.clocks);
        assert!(!wasi.random);
        assert!(!wasi.grants_filesystem());
        assert!(!wasi.grants_ambient_authority());
        assert!(wasi.validate().is_ok());
    }

    #[test]
    fn wasi_pipeline_records_visible_authority() {
        let wasi = WasiSpec::new()
            .arg("--input")
            .arg("/data/input.csv")
            .env("TZ", "UTC")
            .preopen("/data", "/safe/data", true)
            .stdio(StdioMode::Empty, StdioMode::Capture, StdioMode::Capture);

        assert_eq!(wasi.args.len(), 2);
        assert_eq!(wasi.env.get("TZ").map(String::as_str), Some("UTC"));
        assert_eq!(wasi.preopens[0].guest, "/data");
        assert!(wasi.preopens[0].readonly);
        assert!(wasi.grants_filesystem());
        assert!(!wasi.grants_ambient_authority());
        assert!(wasi.validate().is_ok());
    }

    #[test]
    fn wasi_validation_rejects_ambiguous_capability_grants() {
        let err = WasiSpec::new()
            .env("BAD=NAME", "value")
            .validate()
            .expect_err("env name with '=' is ambiguous");
        assert!(err.message.contains("env names"));

        let err = WasiSpec::new()
            .preopen("relative", "/safe/data", true)
            .validate()
            .expect_err("guest preopen path must be absolute");
        assert!(err.message.contains("guest path"));

        let err = WasiSpec::new()
            .stdout_file("")
            .validate()
            .expect_err("file mode needs a file path");
        assert!(err.message.contains("stdout file"));
    }

    #[test]
    fn wasi_stdin_bytes_accept_embedded_nul() {
        let wasi = WasiSpec::new().stdin_bytes(vec![b'A', 0, b'B']);
        assert_eq!(wasi.stdin, StdioMode::String);
        assert_eq!(wasi.stdin_bytes.as_deref(), Some(&[b'A', 0, b'B'][..]));
        assert!(wasi.validate().is_ok());
    }

    #[test]
    fn wasi_inherited_stdio_and_network_are_visible_ambient_authority() {
        let wasi = WasiSpec::new()
            .stdio(StdioMode::Inherit, StdioMode::Capture, StdioMode::Capture)
            .network(true);
        assert!(wasi.grants_ambient_authority());
        assert!(wasi.validate().is_ok());
    }
}
