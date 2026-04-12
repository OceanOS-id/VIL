use serde_json::{json, Value};

pub fn validate_schema(args: &[Value]) -> Result<Value, String> {
    let data = args.get(0).ok_or("validate_schema: data required")?;
    let schema = args.get(1).ok_or("validate_schema: schema required")?;
    let compiled = jsonschema::JSONSchema::compile(schema)
        .map_err(|e| format!("validate_schema: invalid schema: {}", e))?;
    let result = compiled.validate(data);
    match result {
        Ok(()) => Ok(json!({"valid": true, "errors": []})),
        Err(errors) => {
            let errs: Vec<Value> = errors
                .map(|e| {
                    json!({"path": e.instance_path.to_string(), "message": e.to_string()})
                })
                .collect();
            Ok(json!({"valid": false, "errors": errs}))
        }
    }
}

pub fn register_functions() -> Vec<(&'static str, fn(&[Value]) -> Result<Value, String>)> {
    vec![("validate_schema", validate_schema)]
}
