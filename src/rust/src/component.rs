use std::collections::BTreeMap;

use crate::app::{AppSpec, ArrayPolicy, PreparedApp, Result, RwasmtimeError, WitSpec};
use crate::callbacks::CallbackSet;
use crate::limits::Limits;
use crate::wasi::WasiSpec;
use crate::{Runtime, RuntimeSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentSpec {
    pub source: String,
    pub runtime: Option<RuntimeSpec>,
    pub wasi: Option<WasiSpec>,
    pub limits: Option<Limits>,
    pub callbacks: Option<CallbackSet>,
    pub arrays: ArrayPolicy,
    pub wit: Option<WitSpec>,
}

impl ComponentSpec {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            runtime: None,
            wasi: None,
            limits: None,
            callbacks: None,
            arrays: ArrayPolicy::default(),
            wit: None,
        }
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
        let mut app = AppSpec::new(self.source).component().arrays(self.arrays);
        if let Some(runtime) = self.runtime {
            app = app.runtime_spec(runtime);
        }
        if let Some(wasi) = self.wasi {
            app = app.wasi(wasi);
        }
        if let Some(limits) = self.limits {
            app = app.limits(limits);
        }
        if let Some(callbacks) = self.callbacks {
            app = app.callbacks(callbacks);
        }
        if let Some(wit) = self.wit {
            app.wit(wit.path, wit.world, wit.validate).prepare()
        } else {
            app.prepare()
        }
    }

    pub fn exports(&self) -> Result<Vec<ComponentItem>> {
        Err(RwasmtimeError::not_implemented("wt_component_exports"))
    }

    pub fn imports(&self) -> Result<Vec<ComponentItem>> {
        Err(RwasmtimeError::not_implemented("wt_component_imports"))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentItemKind {
    Function,
    Interface,
    Resource,
    World,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentItem {
    pub name: String,
    pub interface: Option<String>,
    pub kind: ComponentItemKind,
    pub params_schema: Option<String>,
    pub results_schema: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentCallRequest {
    pub export: String,
    pub args: Vec<WitValue>,
    pub expected_results: Option<WitType>,
}

impl ComponentCallRequest {
    pub fn new(export: impl Into<String>) -> Self {
        Self {
            export: export.into(),
            args: Vec::new(),
            expected_results: None,
        }
    }

    pub fn arg(mut self, value: WitValue) -> Self {
        self.args.push(value);
        self
    }

    pub fn expected_results(mut self, ty: WitType) -> Self {
        self.expected_results = Some(ty);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitField {
    pub name: String,
    pub ty: WitType,
}

impl WitField {
    pub fn new(name: impl Into<String>, ty: WitType) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitCase {
    pub name: String,
    pub ty: Option<WitType>,
}

impl WitCase {
    pub fn new(name: impl Into<String>, ty: Option<WitType>) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WitType {
    Bool,
    S32,
    S64,
    F32,
    F64,
    String,
    List(Box<WitType>),
    Option(Box<WitType>),
    Tuple(Vec<WitType>),
    Record(Vec<WitField>),
    Enum(Vec<String>),
    Variant(Vec<WitCase>),
    Result {
        ok: Option<Box<WitType>>,
        err: Option<Box<WitType>>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum WitValue {
    Bool(bool),
    S32(i32),
    S64(i64),
    F32(f32),
    F64(f64),
    String(String),
    List(Vec<WitValue>),
    Option(Option<Box<WitValue>>),
    Tuple(Vec<WitValue>),
    Record(BTreeMap<String, WitValue>),
    Enum(String),
    Variant {
        case: String,
        value: Option<Box<WitValue>>,
    },
    ResultOk(Option<Box<WitValue>>),
    ResultErr(Option<Box<WitValue>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WitValueMismatch {
    pub path: String,
    pub expected: String,
    pub found: String,
}

impl WitValueMismatch {
    fn new(path: String, expected: String, found: String) -> Self {
        Self {
            path,
            expected,
            found,
        }
    }
}

impl WitType {
    pub fn list(item: WitType) -> Self {
        Self::List(Box::new(item))
    }
    pub fn option(item: WitType) -> Self {
        Self::Option(Box::new(item))
    }

    pub fn matches_value(&self, value: &WitValue) -> bool {
        self.validate_value(value).is_ok()
    }

    pub fn validate_value(&self, value: &WitValue) -> std::result::Result<(), WitValueMismatch> {
        validate_wit_value(self, value, "$".to_string())
    }
}

impl WitValue {
    pub fn type_label(&self) -> &'static str {
        match self {
            WitValue::Bool(_) => "bool",
            WitValue::S32(_) => "s32",
            WitValue::S64(_) => "s64",
            WitValue::F32(_) => "f32",
            WitValue::F64(_) => "f64",
            WitValue::String(_) => "string",
            WitValue::List(_) => "list",
            WitValue::Option(_) => "option",
            WitValue::Tuple(_) => "tuple",
            WitValue::Record(_) => "record",
            WitValue::Enum(_) => "enum",
            WitValue::Variant { .. } => "variant",
            WitValue::ResultOk(_) => "result.ok",
            WitValue::ResultErr(_) => "result.err",
        }
    }
}

fn validate_wit_value(
    ty: &WitType,
    value: &WitValue,
    path: String,
) -> std::result::Result<(), WitValueMismatch> {
    match (ty, value) {
        (WitType::Bool, WitValue::Bool(_))
        | (WitType::S32, WitValue::S32(_))
        | (WitType::S64, WitValue::S64(_))
        | (WitType::F32, WitValue::F32(_))
        | (WitType::F64, WitValue::F64(_))
        | (WitType::String, WitValue::String(_)) => Ok(()),
        (WitType::List(item_ty), WitValue::List(items)) => {
            for (i, item) in items.iter().enumerate() {
                validate_wit_value(item_ty, item, format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        (WitType::Option(item_ty), WitValue::Option(Some(item))) => {
            validate_wit_value(item_ty, item, format!("{path}?"))
        }
        (WitType::Option(_), WitValue::Option(None)) => Ok(()),
        (WitType::Tuple(types), WitValue::Tuple(values)) if types.len() == values.len() => {
            for (i, (item_ty, item)) in types.iter().zip(values.iter()).enumerate() {
                validate_wit_value(item_ty, item, format!("{path}.{i}"))?;
            }
            Ok(())
        }
        (WitType::Record(fields), WitValue::Record(values)) => {
            for field in fields {
                let item = values.get(&field.name).ok_or_else(|| {
                    WitValueMismatch::new(
                        format!("{path}.{}", field.name),
                        describe_wit_type(&field.ty),
                        "missing".to_string(),
                    )
                })?;
                validate_wit_value(&field.ty, item, format!("{path}.{}", field.name))?;
            }
            Ok(())
        }
        (WitType::Enum(cases), WitValue::Enum(case)) if cases.iter().any(|c| c == case) => Ok(()),
        (WitType::Variant(cases), WitValue::Variant { case, value }) => {
            let item = cases.iter().find(|c| c.name == *case).ok_or_else(|| {
                WitValueMismatch::new(
                    path.clone(),
                    describe_wit_type(ty),
                    value_label(value.as_deref(), "unknown-variant-case"),
                )
            })?;
            match (&item.ty, value) {
                (None, None) => Ok(()),
                (Some(item_ty), Some(item_value)) => {
                    validate_wit_value(item_ty, item_value, format!("{path}.{case}"))
                }
                (None, Some(_)) => Err(WitValueMismatch::new(
                    path,
                    "case without payload".to_string(),
                    "payload".to_string(),
                )),
                (Some(item_ty), None) => Err(WitValueMismatch::new(
                    path,
                    describe_wit_type(item_ty),
                    "missing".to_string(),
                )),
            }
        }
        (WitType::Result { ok, .. }, WitValue::ResultOk(value)) => {
            validate_optional_payload(ok.as_deref(), value.as_deref(), format!("{path}.ok"))
        }
        (WitType::Result { err, .. }, WitValue::ResultErr(value)) => {
            validate_optional_payload(err.as_deref(), value.as_deref(), format!("{path}.err"))
        }
        _ => Err(WitValueMismatch::new(
            path,
            describe_wit_type(ty),
            value.type_label().to_string(),
        )),
    }
}

fn validate_optional_payload(
    ty: Option<&WitType>,
    value: Option<&WitValue>,
    path: String,
) -> std::result::Result<(), WitValueMismatch> {
    match (ty, value) {
        (None, None) => Ok(()),
        (Some(ty), Some(value)) => validate_wit_value(ty, value, path),
        (None, Some(value)) => Err(WitValueMismatch::new(
            path,
            "no payload".to_string(),
            value.type_label().to_string(),
        )),
        (Some(ty), None) => Err(WitValueMismatch::new(
            path,
            describe_wit_type(ty),
            "missing".to_string(),
        )),
    }
}

fn value_label(value: Option<&WitValue>, fallback: &str) -> String {
    value
        .map(WitValue::type_label)
        .unwrap_or(fallback)
        .to_string()
}

fn describe_wit_type(ty: &WitType) -> String {
    match ty {
        WitType::Bool => "bool".to_string(),
        WitType::S32 => "s32".to_string(),
        WitType::S64 => "s64".to_string(),
        WitType::F32 => "f32".to_string(),
        WitType::F64 => "f64".to_string(),
        WitType::String => "string".to_string(),
        WitType::List(item) => format!("list<{}>", describe_wit_type(item)),
        WitType::Option(item) => format!("option<{}>", describe_wit_type(item)),
        WitType::Tuple(items) => format!(
            "tuple<{}>",
            items
                .iter()
                .map(describe_wit_type)
                .collect::<Vec<_>>()
                .join(",")
        ),
        WitType::Record(_) => "record".to_string(),
        WitType::Enum(_) => "enum".to_string(),
        WitType::Variant(_) => "variant".to_string(),
        WitType::Result { .. } => "result".to_string(),
    }
}

impl ComponentItem {
    pub fn function(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            interface: None,
            kind: ComponentItemKind::Function,
            params_schema: None,
            results_schema: None,
        }
    }

    pub fn interface(mut self, value: impl Into<String>) -> Self {
        self.interface = Some(value.into());
        self
    }
    pub fn params_schema(mut self, value: impl Into<String>) -> Self {
        self.params_schema = Some(value.into());
        self
    }
    pub fn results_schema(mut self, value: impl Into<String>) -> Self {
        self.results_schema = Some(value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{ArrayDType, ArrayPolicy, ArrayTransport, RwasmtimeErrorKind};
    use crate::callbacks::{CallbackSet, CallbackSpec};
    use crate::config::{CompilerSpec, RuntimeSpec};
    use crate::limits::Limits;
    use crate::wasi::WasiSpec;

    #[test]
    fn component_pipeline_prepares_as_component_app() {
        let runtime = RuntimeSpec::new()
            .compiler(CompilerSpec::cranelift())
            .build();
        let callbacks =
            CallbackSet::new().callback(CallbackSpec::component("rwasmtime:host/callbacks.log"));
        let prepared = ComponentSpec::new("stats_plugin.component.wasm")
            .runtime(&runtime)
            .wasi(WasiSpec::new().arg("--mode=test"))
            .limits(Limits::new().memory_bytes(128 * 1024 * 1024))
            .callbacks(callbacks)
            .arrays(
                ArrayPolicy::new()
                    .default_dtype(ArrayDType::F64)
                    .transport(ArrayTransport::Arena),
            )
            .wit("world.wit", Some("stats".to_string()), true)
            .prepare();

        assert_eq!(prepared.spec.source, "stats_plugin.component.wasm");
        assert_eq!(prepared.spec.kind, crate::app::SourceKind::Component);
        assert_eq!(prepared.spec.wasi.as_ref().map(|w| w.args.len()), Some(1));
        assert_eq!(
            prepared.spec.limits.as_ref().and_then(|l| l.memory_bytes),
            Some(128 * 1024 * 1024)
        );
        assert_eq!(
            prepared.spec.callbacks.as_ref().map(|c| c.imports.len()),
            Some(1)
        );
        assert_eq!(
            prepared.spec.wit.as_ref().and_then(|w| w.world.as_deref()),
            Some("stats")
        );
    }

    #[test]
    fn component_introspection_fails_honestly_until_wit_backend_lands() {
        let component =
            ComponentSpec::new("stats_plugin.component.wasm").wit("world.wit", None, true);
        let err = component
            .exports()
            .expect_err("component introspection is backend work");
        assert_eq!(err.kind, RwasmtimeErrorKind::NotImplemented);
        assert!(err.message.contains("wt_component_exports"));

        let err = component
            .imports()
            .expect_err("component introspection is backend work");
        assert_eq!(err.kind, RwasmtimeErrorKind::NotImplemented);
        assert!(err.message.contains("wt_component_imports"));
    }

    #[test]
    fn component_item_records_wit_function_shape() {
        let item = ComponentItem::function("run")
            .interface("rwasmtime:stats")
            .params_schema("list<f64>")
            .results_schema("f64");

        assert_eq!(item.kind, ComponentItemKind::Function);
        assert_eq!(item.interface.as_deref(), Some("rwasmtime:stats"));
        assert_eq!(item.params_schema.as_deref(), Some("list<f64>"));
        assert_eq!(item.results_schema.as_deref(), Some("f64"));
    }

    #[test]
    fn wit_values_validate_against_copied_dynamic_types() {
        let ty = WitType::Record(vec![
            WitField::new("name", WitType::String),
            WitField::new("scores", WitType::list(WitType::F64)),
            WitField::new("label", WitType::option(WitType::String)),
        ]);
        let mut record = BTreeMap::new();
        record.insert("name".to_string(), WitValue::String("sample".to_string()));
        record.insert(
            "scores".to_string(),
            WitValue::List(vec![WitValue::F64(1.0), WitValue::F64(2.0)]),
        );
        record.insert("label".to_string(), WitValue::Option(None));

        assert!(ty.matches_value(&WitValue::Record(record)));
    }

    #[test]
    fn wit_value_validation_reports_nested_mismatch_paths() {
        let ty = WitType::list(WitType::Record(vec![WitField::new("score", WitType::F64)]));
        let mut record = BTreeMap::new();
        record.insert("score".to_string(), WitValue::String("bad".to_string()));
        let err = ty
            .validate_value(&WitValue::List(vec![WitValue::Record(record)]))
            .expect_err("string is not f64");

        assert_eq!(err.path, "$[0].score");
        assert_eq!(err.expected, "f64");
        assert_eq!(err.found, "string");
    }

    #[test]
    fn component_call_request_records_wit_args_without_r_objects() {
        let request = ComponentCallRequest::new("stats:run")
            .arg(WitValue::List(vec![WitValue::F64(1.0), WitValue::F64(2.0)]))
            .expected_results(WitType::Result {
                ok: Some(Box::new(WitType::F64)),
                err: Some(Box::new(WitType::String)),
            });

        assert_eq!(request.export, "stats:run");
        assert_eq!(request.args.len(), 1);
        assert!(matches!(
            request.expected_results,
            Some(WitType::Result { .. })
        ));
    }
}
