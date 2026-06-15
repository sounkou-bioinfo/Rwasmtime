use crate::app::{Result, RwasmtimeError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplProtocol {
    Component,
    Stdio,
    Callback,
    CoreMemory,
    Mock,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplSpec {
    pub protocol: ReplProtocol,
    pub eval_export: Option<String>,
    pub prompt: String,
    pub continuation: String,
    pub guest: Option<String>,
}

impl ReplSpec {
    pub fn component(eval_export: impl Into<String>) -> Self {
        Self {
            protocol: ReplProtocol::Component,
            eval_export: Some(eval_export.into()),
            prompt: "> ".to_string(),
            continuation: "+ ".to_string(),
            guest: None,
        }
    }

    pub fn stdio() -> Self {
        Self {
            protocol: ReplProtocol::Stdio,
            eval_export: None,
            prompt: "> ".to_string(),
            continuation: "+ ".to_string(),
            guest: None,
        }
    }

    pub fn callback(eval_export: impl Into<String>) -> Self {
        Self {
            protocol: ReplProtocol::Callback,
            eval_export: Some(eval_export.into()),
            prompt: "> ".to_string(),
            continuation: "+ ".to_string(),
            guest: None,
        }
    }

    pub fn core_memory(eval_export: impl Into<String>) -> Self {
        Self {
            protocol: ReplProtocol::CoreMemory,
            eval_export: Some(eval_export.into()),
            prompt: "> ".to_string(),
            continuation: "+ ".to_string(),
            guest: None,
        }
    }

    pub fn mock() -> Self {
        Self {
            protocol: ReplProtocol::Mock,
            eval_export: None,
            prompt: "> ".to_string(),
            continuation: "+ ".to_string(),
            guest: Some("mock".to_string()),
        }
    }

    pub fn guest(mut self, name: impl Into<String>) -> Self {
        self.guest = Some(name.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>, continuation: impl Into<String>) -> Self {
        self.prompt = prompt.into();
        self.continuation = continuation.into();
        self
    }

    pub fn validate(&self) -> Result<()> {
        if self.prompt.is_empty() {
            return Err(RwasmtimeError::invalid_argument(
                "REPL prompt must not be empty",
            ));
        }
        if self.continuation.is_empty() {
            return Err(RwasmtimeError::invalid_argument(
                "REPL continuation prompt must not be empty",
            ));
        }
        match self.protocol {
            ReplProtocol::Component | ReplProtocol::Callback | ReplProtocol::CoreMemory => {
                if self.eval_export.as_deref().unwrap_or("").is_empty() {
                    return Err(RwasmtimeError::invalid_argument(
                        "component/callback/core REPL protocols require eval_export",
                    ));
                }
            }
            ReplProtocol::Stdio => {}
            ReplProtocol::Mock => {
                if self.guest.as_deref() != Some("mock") {
                    return Err(RwasmtimeError::invalid_argument(
                        "mock REPL protocol is reserved for scaffold tests",
                    ));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplRequest {
    pub sequence: u64,
    pub code: String,
}

impl ReplRequest {
    pub fn new(sequence: u64, code: impl Into<String>) -> Self {
        Self {
            sequence,
            code: code.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplResult {
    pub input: String,
    pub stdout: String,
    pub stderr: String,
    pub value: Option<String>,
    pub error: Option<String>,
    pub status: i32,
    pub complete: bool,
}

impl ReplResult {
    pub fn pending(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            stdout: String::new(),
            stderr: String::new(),
            value: None,
            error: None,
            status: 0,
            complete: false,
        }
    }

    pub fn mock(input: impl Into<String>) -> Self {
        let input = input.into();
        Self {
            input: input.clone(),
            stdout: format!("<mock sandbox> {input}"),
            stderr: String::new(),
            value: None,
            error: None,
            status: 0,
            complete: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplSession {
    pub spec: ReplSpec,
    pub open: bool,
    pub history: Vec<ReplRequest>,
    pub results: Vec<ReplResult>,
}

impl ReplSession {
    pub fn new(spec: ReplSpec) -> Result<Self> {
        spec.validate()?;
        Ok(Self {
            spec,
            open: true,
            history: Vec::new(),
            results: Vec::new(),
        })
    }

    pub fn close(mut self) -> Self {
        self.open = false;
        self
    }

    pub fn send(&mut self, code: impl Into<String>) -> Result<()> {
        if !self.open {
            return Err(RwasmtimeError::invalid_argument("REPL is closed"));
        }
        let code = code.into();
        match self.spec.protocol {
            ReplProtocol::Mock => {
                let sequence = self.history.len() as u64 + 1;
                self.history.push(ReplRequest::new(sequence, code.clone()));
                self.results.push(ReplResult::mock(code));
                Ok(())
            }
            ReplProtocol::Component => Err(RwasmtimeError::not_implemented(
                "wt_repl_send protocol=component",
            )),
            ReplProtocol::Stdio => Err(RwasmtimeError::not_implemented(
                "wt_repl_send protocol=stdio",
            )),
            ReplProtocol::Callback => Err(RwasmtimeError::not_implemented(
                "wt_repl_send protocol=callback",
            )),
            ReplProtocol::CoreMemory => Err(RwasmtimeError::not_implemented(
                "wt_repl_send protocol=core",
            )),
        }
    }

    pub fn eval(&mut self, code: impl Into<String>) -> Result<ReplResult> {
        self.send(code)?;
        Ok(self
            .results
            .last()
            .cloned()
            .expect("mock send stores a result"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::RwasmtimeErrorKind;

    #[test]
    fn webr_repl_is_a_guest_protocol_not_host_eval() {
        let spec = ReplSpec::component("webr:host/repl.eval").guest("webR");
        assert_eq!(spec.protocol, ReplProtocol::Component);
        assert_eq!(spec.guest.as_deref(), Some("webR"));
        assert_eq!(spec.eval_export.as_deref(), Some("webr:host/repl.eval"));
        assert!(spec.validate().is_ok());
    }

    #[test]
    fn repl_protocol_validation_requires_explicit_guest_contract() {
        let err = ReplSpec::component("")
            .validate()
            .expect_err("component REPL needs an export");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("eval_export"));

        let err = ReplSpec::core_memory("")
            .validate()
            .expect_err("core-memory REPL needs an export");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("eval_export"));

        let err = ReplSpec::mock()
            .guest("webR")
            .validate()
            .expect_err("mock is not webR");
        assert_eq!(err.kind, RwasmtimeErrorKind::InvalidArgument);
        assert!(err.message.contains("mock REPL"));
    }

    #[test]
    fn non_mock_repl_send_fails_without_recording_history() {
        let mut session = ReplSession::new(ReplSpec::component("guest:repl/eval")).unwrap();
        let err = session
            .send("1 + 1")
            .expect_err("backend protocol is not implemented");
        assert_eq!(err.kind, RwasmtimeErrorKind::NotImplemented);
        assert!(session.history.is_empty());
        assert!(session.results.is_empty());
    }

    #[test]
    fn mock_repl_records_inputs_without_host_eval() {
        let mut session = ReplSession::new(ReplSpec::mock()).unwrap();
        let result = session.eval("1 + 1").unwrap();
        assert_eq!(session.history.len(), 1);
        assert_eq!(session.history[0].code, "1 + 1");
        assert_eq!(result.stdout, "<mock sandbox> 1 + 1");
        assert_eq!(result.value, None);
        assert_eq!(result.error, None);
        assert_eq!(result.status, 0);
        assert!(result.complete);
    }
}
