use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::app::{Result as WtResult, RwasmtimeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackMode {
    Blocking,
    FireAndForget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackPolicy {
    pub mode: CallbackMode,
    pub timeout_ms: Option<u64>,
    pub max_depth: u32,
    pub reentrant: bool,
}

impl CallbackPolicy {
    pub fn blocking_main_thread() -> Self {
        Self { mode: CallbackMode::Blocking, timeout_ms: None, max_depth: 1, reentrant: false }
    }

    pub fn fire_and_forget() -> Self {
        Self { mode: CallbackMode::FireAndForget, timeout_ms: None, max_depth: 1, reentrant: false }
    }

    pub fn timeout_ms(mut self, value: u64) -> Self { self.timeout_ms = Some(value); self }
    pub fn max_depth(mut self, value: u32) -> Self { self.max_depth = value; self }
    pub fn reentrant(mut self, value: bool) -> Self { self.reentrant = value; self }

    pub fn validate(&self) -> WtResult<()> {
        if self.max_depth == 0 {
            return Err(RwasmtimeError::invalid_argument("callback policy max_depth must be at least 1"));
        }
        if self.reentrant && self.max_depth < 2 {
            return Err(RwasmtimeError::invalid_argument("reentrant callback policy requires max_depth of at least 2"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackWakeStrategy {
    MainThreadImmediate,
    PosixInputHandlerPipe,
    WindowsMessagePump,
    AdapterManaged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallbackReturnPath {
    FireAndForget,
    BlockingReply,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallbackServicePlan {
    pub wake_strategy: CallbackWakeStrategy,
    pub return_path: CallbackReturnPath,
    pub user_drain_required: bool,
}

impl CallbackServicePlan {
    pub fn for_target(policy: &CallbackPolicy, target_os: &str, on_main_thread: bool) -> Self {
        let wake_strategy = if on_main_thread {
            CallbackWakeStrategy::MainThreadImmediate
        } else {
            match target_os {
                "linux" | "macos" | "freebsd" | "openbsd" | "netbsd" => CallbackWakeStrategy::PosixInputHandlerPipe,
                "windows" => CallbackWakeStrategy::WindowsMessagePump,
                _ => CallbackWakeStrategy::AdapterManaged,
            }
        };
        let return_path = match policy.mode {
            CallbackMode::Blocking => CallbackReturnPath::BlockingReply,
            CallbackMode::FireAndForget => CallbackReturnPath::FireAndForget,
        };
        Self { wake_strategy, return_path, user_drain_required: false }
    }

    pub fn requires_user_drain(&self) -> bool {
        self.user_drain_required
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackAbi {
    Component,
    Core,
    CoreMsgpack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackSpec {
    pub module: Option<String>,
    pub name: String,
    pub abi: CallbackAbi,
    pub params_schema: Option<String>,
    pub results_schema: Option<String>,
    pub policy: CallbackPolicy,
}

impl CallbackSpec {
    pub fn component(name: impl Into<String>) -> Self {
        Self {
            module: None,
            name: name.into(),
            abi: CallbackAbi::Component,
            params_schema: None,
            results_schema: None,
            policy: CallbackPolicy::blocking_main_thread(),
        }
    }

    pub fn core(module: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            module: Some(module.into()),
            name: name.into(),
            abi: CallbackAbi::Core,
            params_schema: None,
            results_schema: None,
            policy: CallbackPolicy::blocking_main_thread(),
        }
    }

    pub fn core_msgpack(module: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            module: Some(module.into()),
            name: name.into(),
            abi: CallbackAbi::CoreMsgpack,
            params_schema: None,
            results_schema: None,
            policy: CallbackPolicy::blocking_main_thread(),
        }
    }

    pub fn params_schema(mut self, value: impl Into<String>) -> Self { self.params_schema = Some(value.into()); self }
    pub fn results_schema(mut self, value: impl Into<String>) -> Self { self.results_schema = Some(value.into()); self }
    pub fn policy(mut self, value: CallbackPolicy) -> Self { self.policy = value; self }

    pub fn key(&self) -> String {
        match &self.module {
            Some(module) => format!("{module}::{}", self.name),
            None => self.name.clone(),
        }
    }

    pub fn validate(&self) -> WtResult<()> {
        validate_non_empty_name("callback name", &self.name)?;
        if let Some(schema) = &self.params_schema {
            validate_non_empty_name("callback params schema", schema)?;
        }
        if let Some(schema) = &self.results_schema {
            validate_non_empty_name("callback results schema", schema)?;
        }
        match self.abi {
            CallbackAbi::Component => {
                if self.module.is_some() {
                    return Err(RwasmtimeError::invalid_argument("component callbacks must not set a core module name"));
                }
            }
            CallbackAbi::Core | CallbackAbi::CoreMsgpack => {
                let Some(module) = &self.module else {
                    return Err(RwasmtimeError::invalid_argument("core callbacks require a module name"));
                };
                validate_non_empty_name("callback module", module)?;
            }
        }
        if self.policy.mode == CallbackMode::FireAndForget && self.results_schema.is_some() {
            return Err(RwasmtimeError::invalid_argument("fire-and-forget callbacks must not declare results"));
        }
        self.policy.validate()
    }
}

fn validate_non_empty_name(label: &str, value: &str) -> WtResult<()> {
    if value.is_empty() || value.contains('\0') {
        return Err(RwasmtimeError::invalid_argument(format!("{label} must be non-empty and must not contain NUL bytes")));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackSet {
    pub imports: Vec<CallbackSpec>,
}

impl CallbackSet {
    pub fn new() -> Self {
        Self { imports: Vec::new() }
    }

    pub fn callback(mut self, spec: CallbackSpec) -> Self {
        self.imports.push(spec);
        self
    }

    pub fn validate(&self) -> WtResult<()> {
        let mut seen = HashSet::new();
        for spec in &self.imports {
            spec.validate()?;
            let key = spec.key();
            if !seen.insert(key.clone()) {
                return Err(RwasmtimeError::invalid_argument(format!("duplicate callback import: {key}")));
            }
        }
        Ok(())
    }
}

impl Default for CallbackSet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CallbackTicket(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackRequest {
    pub id: String,
    pub name: String,
    pub payload: Vec<u8>,
    pub metadata: BTreeMap<String, String>,
}

impl CallbackRequest {
    pub fn new(id: impl Into<String>, name: impl Into<String>, payload: Vec<u8>) -> Self {
        Self { id: id.into(), name: name.into(), payload, metadata: BTreeMap::new() }
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackReply {
    pub payload: Vec<u8>,
}

impl CallbackReply {
    pub fn new(payload: Vec<u8>) -> Self {
        Self { payload }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackError {
    pub message: String,
}

impl CallbackError {
    pub fn new(message: impl Into<String>) -> Self {
        Self { message: message.into() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallbackCompletion {
    Reply(CallbackReply),
    Error(CallbackError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCallback {
    pub ticket: CallbackTicket,
    pub request: CallbackRequest,
    pub policy: CallbackPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallbackBrokerQueue {
    next_ticket: u64,
    pending: VecDeque<PendingCallback>,
    completed: HashMap<CallbackTicket, CallbackCompletion>,
}

impl CallbackBrokerQueue {
    pub fn new() -> Self {
        Self { next_ticket: 1, pending: VecDeque::new(), completed: HashMap::new() }
    }

    pub fn enqueue(&mut self, request: CallbackRequest, policy: CallbackPolicy) -> CallbackTicket {
        let ticket = CallbackTicket(self.next_ticket);
        self.next_ticket += 1;
        self.pending.push_back(PendingCallback { ticket, request, policy });
        ticket
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }

    pub fn completed_len(&self) -> usize {
        self.completed.len()
    }

    pub fn pop_pending(&mut self) -> Option<PendingCallback> {
        self.pending.pop_front()
    }

    pub fn complete(&mut self, ticket: CallbackTicket, reply: CallbackReply) {
        self.completed.insert(ticket, CallbackCompletion::Reply(reply));
    }

    pub fn fail(&mut self, ticket: CallbackTicket, error: CallbackError) {
        self.completed.insert(ticket, CallbackCompletion::Error(error));
    }

    pub fn take_completion(&mut self, ticket: CallbackTicket) -> Option<CallbackCompletion> {
        self.completed.remove(&ticket)
    }

    pub fn service_pending<F>(&mut self, max: usize, mut host: F) -> usize
    where
        F: FnMut(&CallbackRequest, &CallbackPolicy) -> Result<CallbackReply, CallbackError>,
    {
        let mut serviced = 0;
        while serviced < max {
            let Some(pending) = self.pop_pending() else { break; };
            match host(&pending.request, &pending.policy) {
                Ok(reply) => self.complete(pending.ticket, reply),
                Err(error) => self.fail(pending.ticket, error),
            }
            serviced += 1;
        }
        serviced
    }
}

impl Default for CallbackBrokerQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub trait HostCallbackBroker: Send + Sync + 'static {
    fn call_blocking(
        &self,
        request: CallbackRequest,
        policy: CallbackPolicy,
    ) -> Result<CallbackReply, CallbackError>;

    fn enqueue(&self, request: CallbackRequest, policy: CallbackPolicy) -> Result<CallbackTicket, CallbackError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_callback_policy_is_non_reentrant() {
        let policy = CallbackPolicy::blocking_main_thread().timeout_ms(1000);
        assert_eq!(policy.timeout_ms, Some(1000));
        assert!(!policy.reentrant);
        assert_eq!(policy.max_depth, 1);
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn callback_policy_validation_rejects_ambiguous_reentrancy() {
        let err = CallbackPolicy::blocking_main_thread()
            .max_depth(0)
            .validate()
            .expect_err("depth zero is ambiguous");
        assert!(err.message.contains("at least 1"));

        let err = CallbackPolicy::blocking_main_thread()
            .max_depth(1)
            .reentrant(true)
            .validate()
            .expect_err("reentrant depth one is unsafe");
        assert!(err.message.contains("at least 2"));
    }

    #[test]
    fn callback_set_records_import_metadata_without_r_closures() {
        let callbacks = CallbackSet::new()
            .callback(
                CallbackSpec::component("rwasmtime:host/callbacks.log")
                    .params_schema("msg:string")
                    .policy(CallbackPolicy::blocking_main_thread()),
            )
            .callback(
                CallbackSpec::core("r", "score_f64")
                    .params_schema("f64,f64")
                    .results_schema("f64"),
            );

        assert_eq!(callbacks.imports.len(), 2);
        assert_eq!(callbacks.imports[0].abi, CallbackAbi::Component);
        assert_eq!(callbacks.imports[1].module.as_deref(), Some("r"));
        assert!(callbacks.validate().is_ok());
    }

    #[test]
    fn callback_spec_validation_enforces_abi_and_policy_boundaries() {
        let err = CallbackSpec::component("guest/log")
            .policy(CallbackPolicy::fire_and_forget())
            .results_schema("string")
            .validate()
            .expect_err("fire-and-forget callbacks cannot return values");
        assert!(err.message.contains("fire-and-forget"));

        let err = CallbackSpec {
            module: None,
            name: "score".to_string(),
            abi: CallbackAbi::Core,
            params_schema: None,
            results_schema: None,
            policy: CallbackPolicy::blocking_main_thread(),
        }
        .validate()
        .expect_err("core callbacks require a module");
        assert!(err.message.contains("module"));

        let err = CallbackSpec::core("r", "")
            .validate()
            .expect_err("empty callback names are invalid");
        assert!(err.message.contains("callback name"));
    }

    #[test]
    fn callback_set_validation_rejects_duplicate_imports() {
        let callbacks = CallbackSet::new()
            .callback(CallbackSpec::core("r", "score"))
            .callback(CallbackSpec::core("r", "score"));
        let err = callbacks.validate().expect_err("duplicate imports should fail before linker wiring");
        assert!(err.message.contains("duplicate callback import"));
    }

    #[test]
    fn callback_broker_queue_hands_requests_to_main_thread_service() {
        let mut queue = CallbackBrokerQueue::new();
        let ticket = queue.enqueue(
            CallbackRequest::new("1", "rwasmtime:host/callbacks.score", vec![1, 2, 3])
                .metadata("encoding", "bytes"),
            CallbackPolicy::blocking_main_thread().timeout_ms(1000),
        );

        assert_eq!(queue.pending_len(), 1);
        assert_eq!(queue.completed_len(), 0);

        let serviced = queue.service_pending(10, |request, policy| {
            assert_eq!(request.name, "rwasmtime:host/callbacks.score");
            assert_eq!(request.metadata.get("encoding").map(String::as_str), Some("bytes"));
            assert_eq!(policy.timeout_ms, Some(1000));
            Ok(CallbackReply::new(vec![42]))
        });

        assert_eq!(serviced, 1);
        assert_eq!(queue.pending_len(), 0);
        assert_eq!(queue.completed_len(), 1);
        assert_eq!(queue.take_completion(ticket), Some(CallbackCompletion::Reply(CallbackReply::new(vec![42]))));
        assert_eq!(queue.completed_len(), 0);
    }

    #[test]
    fn callback_broker_queue_records_callback_errors_without_calling_r() {
        let mut queue = CallbackBrokerQueue::new();
        let ticket = queue.enqueue(
            CallbackRequest::new("2", "rwasmtime:host/callbacks.fail", Vec::new()),
            CallbackPolicy::blocking_main_thread(),
        );

        let serviced = queue.service_pending(1, |_request, _policy| Err(CallbackError::new("callback failed")));
        assert_eq!(serviced, 1);
        assert_eq!(queue.take_completion(ticket), Some(CallbackCompletion::Error(CallbackError::new("callback failed"))));
    }

    #[test]
    fn callback_broker_queue_respects_service_budget() {
        let mut queue = CallbackBrokerQueue::new();
        let first = queue.enqueue(CallbackRequest::new("1", "a", Vec::new()), CallbackPolicy::blocking_main_thread());
        let second = queue.enqueue(CallbackRequest::new("2", "b", Vec::new()), CallbackPolicy::blocking_main_thread());

        let serviced = queue.service_pending(1, |request, _policy| Ok(CallbackReply::new(request.name.as_bytes().to_vec())));
        assert_eq!(serviced, 1);
        assert_eq!(queue.pending_len(), 1);
        assert!(matches!(queue.take_completion(first), Some(CallbackCompletion::Reply(_))));
        assert_eq!(queue.take_completion(second), None);
    }

    #[test]
    fn callback_service_plan_uses_platform_wake_machinery_without_user_drain() {
        let blocking = CallbackPolicy::blocking_main_thread();
        let linux = CallbackServicePlan::for_target(&blocking, "linux", false);
        assert_eq!(linux.wake_strategy, CallbackWakeStrategy::PosixInputHandlerPipe);
        assert_eq!(linux.return_path, CallbackReturnPath::BlockingReply);
        assert!(!linux.requires_user_drain());

        let windows = CallbackServicePlan::for_target(&CallbackPolicy::fire_and_forget(), "windows", false);
        assert_eq!(windows.wake_strategy, CallbackWakeStrategy::WindowsMessagePump);
        assert_eq!(windows.return_path, CallbackReturnPath::FireAndForget);
        assert!(!windows.requires_user_drain());

        let main_thread = CallbackServicePlan::for_target(&blocking, "linux", true);
        assert_eq!(main_thread.wake_strategy, CallbackWakeStrategy::MainThreadImmediate);
        assert!(!main_thread.requires_user_drain());
    }
}
