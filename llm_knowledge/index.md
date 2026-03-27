# VIL LLM Knowledge Base

Quick-reference for LLM assistants helping developers build with VIL.

## How to Use
Pick the file matching the user's question. Each file is self-contained.

## File Map

### Quickstart
| File | Description | Keywords |
|------|-------------|----------|
| [quickstart/what-is-vil.md](quickstart/what-is-vil.md) | What VIL is, 101 crates, 49 examples, patterns, key types | overview, introduction, architecture, crates |
| [quickstart/hello-server.md](quickstart/hello-server.md) | Minimal VilApp hello server | getting started, first server, hello world |
| [quickstart/hello-pipeline.md](quickstart/hello-pipeline.md) | Minimal vil_workflow! pipeline | getting started, first pipeline, workflow |

### Patterns
| File | Description | Keywords |
|------|-------------|----------|
| [patterns/vx-app.md](patterns/vx-app.md) | VX_APP pattern: ShmSlice, ServiceCtx, VilResponse | server pattern, handler, extractor, response |
| [patterns/sdk-pipeline.md](patterns/sdk-pipeline.md) | SDK_PIPELINE: vil_workflow!, HttpSink/Source, transform | pipeline pattern, streaming, sse, ndjson |
| [patterns/multi-pipeline.md](patterns/multi-pipeline.md) | Multi-pipeline: fan-out, fan-in, diamond | multi-pipeline, topology, shared heap |

### Server
| File | Description | Keywords |
|------|-------------|----------|
| [server/vilapp.md](server/vilapp.md) | VilApp + ServiceProcess API | app builder, service, port, mesh, profile, run |
| [server/extractors.md](server/extractors.md) | ShmSlice, ServiceCtx, RequestId, ShmContext | extractor, body, state, zero-copy |
| [server/response.md](server/response.md) | VilResponse, VilError, HandlerResult | response, error, status code, RFC 7807 |
| [server/macros.md](server/macros.md) | vil_handler, vil_endpoint, vil_app!, vil_service | macro, attribute, annotation, DSL |
| [server/middleware.md](server/middleware.md) | Built-in middleware stack | health, metrics, CORS, tracing, auth |
| [server/config.md](server/config.md) | Configuration profiles, YAML, ENV overrides | config, profile, dev, staging, prod, env, yaml |

### Pipeline
| File | Description | Keywords |
|------|-------------|----------|
| [pipeline/workflow-macro.md](pipeline/workflow-macro.md) | vil_workflow! syntax and routes | workflow, macro, route, LoanWrite, failover |
| [pipeline/transform.md](pipeline/transform.md) | .transform() on HttpSourceBuilder | filter, map, enrich, validate, callback |
| [pipeline/ndjson.md](pipeline/ndjson.md) | NDJSON streaming pipelines | ndjson, newline, line-by-line, batch |
| [pipeline/sse.md](pipeline/sse.md) | SSE streaming and dialects | sse, openai, anthropic, ollama, json_tap |
| [pipeline/tokens.md](pipeline/tokens.md) | ShmToken vs GenericToken | token, shm, zero-copy, throughput |

### Plugins
| File | Description | Keywords |
|------|-------------|----------|
| [plugins/llm.md](plugins/llm.md) | LLM plugin: chat, streaming, multi-model | llm, chat, completion, openai, anthropic |
| [plugins/rag.md](plugins/rag.md) | RAG plugin: document store, retrieval | rag, retrieval, embedding, vector, search |
| [plugins/agent.md](plugins/agent.md) | Agent plugin: tools, ReAct loop | agent, tool, react, multi-turn |
| [plugins/plugin-trait.md](plugins/plugin-trait.md) | VilPlugin trait and registration | plugin, trait, register, capability |
| [plugins/plugin-sdk.md](plugins/plugin-sdk.md) | vil_plugin_sdk — PluginBuilder, manifest, testing | plugin sdk, builder, manifest, test harness |

### Integrations
| File | Description | Keywords |
|------|-------------|----------|
| [integrations/database.md](integrations/database.md) | sqlx, sea-orm, redis | database, postgres, mysql, sqlite, redis |
| [integrations/messaging.md](integrations/messaging.md) | Kafka, NATS, MQTT | kafka, nats, mqtt, message queue, pub/sub |
| [integrations/graphql.md](integrations/graphql.md) | GraphQL resolver, subscriptions | graphql, resolver, subscription, playground |
| [integrations/grpc.md](integrations/grpc.md) | gRPC gateway, health, protobuf | grpc, protobuf, gateway, health |
| [integrations/websocket.md](integrations/websocket.md) | WebSocket + SSE server-side | websocket, sse, broadcast, hub, topic |
| [integrations/auth.md](integrations/auth.md) | JWT, RBAC, rate limiting, OAuth2 | auth, jwt, rbac, rate limit, cors, oauth |

### Tools
| File | Description | Keywords |
|------|-------------|----------|
| [tools/cli.md](tools/cli.md) | vil CLI commands | cli, init, compile, run, dev, doctor |
| [tools/wasm-faas.md](tools/wasm-faas.md) | WASM FaaS runtime | wasm, capsule, faas, hot-reload, wasi |
| [tools/sidecar.md](tools/sidecar.md) | Sidecar SDK (Python/Go) | sidecar, python, go, uds, pool |
| [tools/custom-code.md](tools/custom-code.md) | 3 execution modes: Native, WASM, Sidecar | custom code, native, wasm, sidecar, exec, failover |

### Recipes
| File | Description | Keywords |
|------|-------------|----------|
| [recipes/rest-crud.md](recipes/rest-crud.md) | Complete CRUD with ServiceCtx + ShmSlice | crud, rest, create, read, update, delete |
| [recipes/ai-gateway.md](recipes/ai-gateway.md) | AI inference gateway with SSE streaming | ai, gateway, inference, llm, streaming |
| [recipes/credit-filter.md](recipes/credit-filter.md) | NDJSON credit filter (NPL detection) | ndjson, credit, filter, npl, fintech |
| [recipes/multi-service.md](recipes/multi-service.md) | Fan-out scatter with shared ExchangeHeap | fan-out, scatter, multi-service, mesh |
