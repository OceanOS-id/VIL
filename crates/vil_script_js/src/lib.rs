//! VIL JavaScript Script Runtime
//!
//! Sandboxed JS execution for workflow tasks and transform nodes.
//! Enable with feature flag: `vil_script_js = { features = ["js"] }`

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub timeout_ms: u64,
    pub max_memory_mb: u64,
    pub allow_net: bool,
    pub allow_fs: bool,
    pub max_output_size_kb: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 10,
            max_memory_mb: 16,
            allow_net: false,
            allow_fs: false,
            max_output_size_kb: 512,
        }
    }
}

pub struct JsRuntime {
    config: SandboxConfig,
    script: Option<String>,
    file_path: Option<String>,
    version: u64,
}

impl JsRuntime {
    pub fn new(config: SandboxConfig) -> Self {
        Self {
            config,
            script: None,
            file_path: None,
            version: 0,
        }
    }

    pub fn load_file(&mut self, path: &str) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read JS script '{}': {}", path, e))?;
        self.script = Some(content);
        self.file_path = Some(path.to_string());
        self.version += 1;
        Ok(())
    }

    pub fn load_inline(&mut self, script: &str) {
        self.script = Some(script.to_string());
        self.version += 1;
    }

    pub fn execute(&self, input: Value) -> Result<Value, String> {
        let script = self.script.as_ref().ok_or("No script loaded")?;
        execute_js(script, &input, &self.config)
    }

    pub fn hot_reload(&mut self) -> Result<u64, String> {
        if let Some(path) = &self.file_path.clone() {
            self.load_file(path)?;
            Ok(self.version)
        } else {
            Err("No file path — inline scripts cannot hot-reload from file".into())
        }
    }

    pub fn version(&self) -> u64 {
        self.version
    }
}

/// Execute JS script with boa_engine.
#[cfg(feature = "js")]
fn execute_js(script: &str, input: &Value, _config: &SandboxConfig) -> Result<Value, String> {
    use boa_engine::{Context, JsValue, Source};

    let mut context = Context::default();

    // Set input global
    let input_json = serde_json::to_string(input).map_err(|e| e.to_string())?;
    let setup = format!(
        "var input = JSON.parse('{}');",
        input_json.replace('\'', "\\'")
    );
    context
        .eval(Source::from_bytes(setup.as_bytes()))
        .map_err(|e| format!("JS input setup: {:?}", e))?;

    // Set ctx global
    context
        .eval(Source::from_bytes(
            b"var ctx = { log: function(){}, requestId: 'test', traceId: 'test' };",
        ))
        .map_err(|e| format!("JS ctx setup: {:?}", e))?;

    // Wrap script to capture return value
    let wrapped = format!("(function() {{ {} }})()", script);
    let result = context
        .eval(Source::from_bytes(wrapped.as_bytes()))
        .map_err(|e| format!("JS execution error: {:?}", e))?;

    js_value_to_json(&result, &mut context)
}

#[cfg(feature = "js")]
fn js_value_to_json(
    val: &boa_engine::JsValue,
    ctx: &mut boa_engine::Context,
) -> Result<Value, String> {
    use boa_engine::JsValue;

    match val {
        JsValue::Null | JsValue::Undefined => Ok(Value::Null),
        JsValue::Boolean(b) => Ok(Value::Bool(*b)),
        JsValue::Integer(i) => Ok(Value::Number(serde_json::Number::from(*i))),
        JsValue::Rational(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .ok_or_else(|| "Invalid float".into()),
        JsValue::String(s) => Ok(Value::String(s.to_std_string_escaped())),
        JsValue::Object(obj) => {
            // Use JSON.stringify to convert
            let json_str = ctx
                .eval(boa_engine::Source::from_bytes(
                    format!("JSON.stringify({})", val.display().to_string()).as_bytes(),
                ))
                .map_err(|e| format!("JSON.stringify error: {:?}", e))?;

            if let JsValue::String(s) = json_str {
                serde_json::from_str(&s.to_std_string_escaped()).map_err(|e| e.to_string())
            } else {
                Ok(Value::Null)
            }
        }
        _ => Ok(Value::Null),
    }
}

/// Stub when js feature is disabled.
#[cfg(not(feature = "js"))]
fn execute_js(_script: &str, input: &Value, _config: &SandboxConfig) -> Result<Value, String> {
    Ok(serde_json::json!({
        "_stub": true,
        "_runtime": "js",
        "_note": "Enable with: vil_script_js = { features = [\"js\"] }",
        "_input_keys": input.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()),
    }))
}
