# vil_plugin_sdk — Community Plugin Interface

Stable API surface for building VIL plugins. Plugin authors depend on `vil_plugin_sdk` only.

## Quick Start

```rust
use vil_plugin_sdk::prelude::*;

pub struct MyPlugin;

impl VilPlugin for MyPlugin {
    fn id(&self) -> &str { "my-plugin" }
    fn version(&self) -> &str { "1.0.0" }

    fn register(&self, ctx: &mut PluginContext) {
        ctx.provide::<String>("greeting", "hello".into());
        ctx.add_service(ServiceProcess::new("my-svc")
            .endpoint(Method::GET, "/hello", get(hello_handler)));
    }
}

// Register in VilApp:
VilApp::new("app").plugin(MyPlugin).run().await;
```

## PluginBuilder (Ergonomic)

```rust
let plugin = PluginBuilder::new("my-plugin", "1.0.0")
    .description("My plugin")
    .on_register(|ctx| {
        ctx.add_service(ServiceProcess::new("my-svc"));
    })
    .build();
```

## PluginManifest (Declarative)

```rust
let manifest = PluginManifest::new("my-plugin", "1.0.0")
    .author("Community")
    .provides("service:my-svc")
    .config_field("api_key", ConfigFieldSchema::string().required().secret());
```

## Testing

```rust
use vil_plugin_sdk::testing::PluginTestHarness;

let mut harness = PluginTestHarness::new();
harness.register(&MyPlugin);
assert_eq!(harness.service_count(), 1);
assert!(harness.has_resource::<String>("greeting"));
```

## Key Types

| Type | Purpose |
|------|---------|
| `VilPlugin` | Core trait: id, version, register, health |
| `PluginContext` | Registration context: add_service, provide/get resources, add_route |
| `PluginBuilder` | Closure-based plugin construction |
| `PluginManifest` | Serializable plugin metadata + config schema |
| `PluginTestHarness` | Unit test without starting server |
| `ResourceRegistry` | Typed DI between plugins |
| `PluginCapability` | Service, Middleware, Resource, CliCommand, DashboardWidget |
| `PluginDependency` | Required or optional dependency on another plugin |
