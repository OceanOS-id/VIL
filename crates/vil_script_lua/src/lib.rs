//! VIL Lua Script Runtime
//!
//! Sandboxed Lua execution for workflow tasks and transform nodes.
//! Used when `code: { mode: script, runtime: lua }` is declared in YAML.
//!
//! Enable with feature flag: `vil_script_lua = { features = ["lua"] }`

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
        Self { timeout_ms: 10, max_memory_mb: 16, allow_net: false, allow_fs: false, max_output_size_kb: 512 }
    }
}

pub struct LuaRuntime {
    config: SandboxConfig,
    script: Option<String>,
    file_path: Option<String>,
    version: u64,
}

impl LuaRuntime {
    pub fn new(config: SandboxConfig) -> Self {
        Self { config, script: None, file_path: None, version: 0 }
    }

    pub fn load_file(&mut self, path: &str) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read Lua script '{}': {}", path, e))?;
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
        execute_lua(script, &input, &self.config)
    }

    pub fn hot_reload(&mut self) -> Result<u64, String> {
        if let Some(path) = &self.file_path.clone() {
            self.load_file(path)?;
            Ok(self.version)
        } else {
            Err("No file path — inline scripts cannot hot-reload from file".into())
        }
    }

    pub fn version(&self) -> u64 { self.version }
}

/// Execute Lua script with input JSON, return output JSON.
#[cfg(feature = "lua")]
fn execute_lua(script: &str, input: &Value, config: &SandboxConfig) -> Result<Value, String> {
    use mlua::{Lua, Result as LuaResult, MultiValue};

    let lua = Lua::new();

    // Set memory limit
    lua.set_memory_limit(config.max_memory_mb as usize * 1024 * 1024)
        .map_err(|e| format!("Lua memory limit: {}", e))?;

    // Remove dangerous globals for sandbox
    if !config.allow_fs {
        lua.globals().set("io", mlua::Value::Nil).ok();
        lua.globals().set("os", mlua::Value::Nil).ok();
    }
    if !config.allow_net {
        lua.globals().set("require", mlua::Value::Nil).ok();
    }

    // Set `input` global as JSON string, parse in Lua
    let input_json = serde_json::to_string(input).map_err(|e| e.to_string())?;
    lua.load(&format!(
        r#"
        -- Minimal JSON decode (for sandbox — no require allowed)
        function vil_json_decode(s)
            -- Use load() to parse JSON-like Lua table literals
            local f = load("return " .. s)
            if f then return f() else return {{}} end
        end
        input = vil_json_decode('{}')
        "#,
        input_json.replace('\\', "\\\\").replace('\'', "\\'")
    )).exec().map_err(|e| format!("Lua input setup: {}", e))?;

    // Set `ctx` global
    lua.load(r#"
        ctx = {
            log = function(level, msg) end,
            request_id = "test",
            trace_id = "test",
        }
    "#).exec().map_err(|e| format!("Lua ctx setup: {}", e))?;

    // Execute the user script
    let result: mlua::Value = lua.load(script).eval()
        .map_err(|e| format!("Lua execution error: {}", e))?;

    // Convert result to JSON
    lua_value_to_json(&result)
}

#[cfg(feature = "lua")]
fn lua_value_to_json(val: &mlua::Value) -> Result<Value, String> {
    match val {
        mlua::Value::Nil => Ok(Value::Null),
        mlua::Value::Boolean(b) => Ok(Value::Bool(*b)),
        mlua::Value::Integer(i) => Ok(Value::Number(serde_json::Number::from(*i))),
        mlua::Value::Number(n) => {
            serde_json::Number::from_f64(*n)
                .map(Value::Number)
                .ok_or_else(|| "Invalid float".into())
        }
        mlua::Value::String(s) => {
            let s = s.to_str().map_err(|e| e.to_string())?;
            Ok(Value::String(s.to_string()))
        }
        mlua::Value::Table(t) => {
            let mut map = serde_json::Map::new();
            for pair in t.clone().pairs::<mlua::Value, mlua::Value>() {
                let (k, v) = pair.map_err(|e| e.to_string())?;
                if let mlua::Value::String(key) = &k {
                    let key = key.to_str().map_err(|e| e.to_string())?;
                    map.insert(key.to_string(), lua_value_to_json(&v)?);
                }
            }
            Ok(Value::Object(map))
        }
        _ => Ok(Value::Null),
    }
}

/// Stub when lua feature is disabled.
#[cfg(not(feature = "lua"))]
fn execute_lua(_script: &str, input: &Value, _config: &SandboxConfig) -> Result<Value, String> {
    Ok(serde_json::json!({
        "_stub": true,
        "_runtime": "lua",
        "_note": "Enable with: vil_script_lua = { features = [\"lua\"] }",
        "_input_keys": input.as_object().map(|o| o.keys().cloned().collect::<Vec<_>>()),
    }))
}
