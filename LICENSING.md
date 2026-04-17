# VIL Ecosystem Licensing Guide

**Version 1.0 — April 2026**

This document defines the licensing terms, permitted uses, and restrictions for all products in the VIL ecosystem.

> **PT RAG Mid Solution** · [vastar.id](https://vastar.id)

---

## 1. Ecosystem Overview

The VIL ecosystem consists of multiple products with distinct licensing models, designed to maximize developer freedom while protecting Vastar's commercial interests in managed workflow services.

| Product | License | Source Code | Cost |
|---------|---------|-------------|------|
| **VIL Libraries** (165+ crates) | Apache 2.0 OR MIT (dual) | Fully open source | Free |
| **VIL Workflow Runtime** (7 crates) | Vastar Source Available License (VSAL) | Source available | Free (with restrictions) |
| **VFlow** | Commercial | Closed source | License fee |
| **VFlow Enterprise** | Commercial | Closed source | License + SLA |
| **VIL IDE** | Proprietary | Closed source | Free |
| **Vastar Cloud Services** | Service Agreement | N/A (SaaS) | Subscription / usage |

---

## 2. VIL Libraries — Apache 2.0 OR MIT (Dual)

### 2.1 Scope

The VIL library layer consists of 165+ Rust crates dual-licensed under the Apache License 2.0 OR the MIT License, at the user's option. You may obtain copies of the licenses at:

- [Apache 2.0](https://www.apache.org/licenses/LICENSE-2.0)
- [MIT](https://opensource.org/licenses/MIT)

#### Crate Categories (Apache/MIT)

**Database connectors** — `vil_db_sqlx`, `vil_db_redis`, `vil_db_mongo`, `vil_db_cassandra`, `vil_db_clickhouse`, `vil_db_dynamodb`, `vil_db_elastic`, `vil_db_neo4j`, `vil_db_timeseries`, `vil_db_sea_orm`, `vil_db_semantic`, `vil_db_macros`

**Message queue connectors** — `vil_mq_nats`, `vil_mq_kafka`, `vil_mq_mqtt`, `vil_mq_rabbitmq`, `vil_mq_pulsar`, `vil_mq_pubsub`, `vil_mq_sqs`

**Storage connectors** — `vil_storage_s3`, `vil_storage_gcs`, `vil_storage_azure`

**Protocol connectors** — `vil_new_http`, `vil_grpc`, `vil_graphql`, `vil_soap`, `vil_ws`, `vil_modbus`, `vil_opcua`, `vil_webhook_out`

**Codec libraries** — `vil_parse_csv`, `vil_parse_xml`, `vil_template`, `vil_json`, `vil_jwt`, `vil_crypto`, `vil_hash`

**Runtime primitives** — `vil_rt`, `vil_types`, `vil_sdk`, `vil_log`, `vil_observer`, `vil_otel`, `vil_obs`, `vil_shm`, `vil_net`, `vil_queue`

**Server library layer** — `vil_server` (umbrella), `vil_server_core`, `vil_server_web`, `vil_server_auth`, `vil_server_config`, `vil_server_db`, `vil_server_format`, `vil_server_macros`, `vil_server_mesh`, `vil_server_test`

**Expression / rule engines** — `vil_expr`, `vil_rules`, `vil_eval`

**ORM** — `vil_orm`, `vil_orm_derive`

**Triggers** — `vil_trigger_core`, `vil_trigger_webhook`, `vil_trigger_cron`, `vil_trigger_kafka`, `vil_trigger_iot`, `vil_trigger_s3`, `vil_trigger_sftp`, `vil_trigger_cdc`, `vil_trigger_db_poll`, `vil_trigger_fs`, `vil_trigger_grpc`, `vil_trigger_email`, `vil_trigger_evm`

**Execution substrates** — `vil_capsule` (WASM sandbox), `vil_sidecar` (Sidecar SDK), `vil_plugin_sdk`

**AI / LLM libraries** — `vil_llm`, `vil_llm_cache`, `vil_llm_proxy`, `vil_rag`, `vil_graphrag`, `vil_private_rag`, `vil_federated_rag`, `vil_realtime_rag`, `vil_streaming_rag`, `vil_embedder`, `vil_vectordb`, `vil_reranker`, `vil_guardrails`, `vil_prompt_shield`, `vil_prompts`, `vil_prompt_optimizer`, `vil_agent`, `vil_multi_agent`, `vil_semantic_router`, `vil_ai_gateway`, `vil_ai_trace`, `vil_ai_compiler`, `vil_inference`, `vil_model_registry`, `vil_model_serving`, `vil_quantized`, `vil_synthetic`, `vil_rlhf_data`, `vil_feature_store`, `vil_multimodal`, `vil_tokenizer`, `vil_reshape`, `vil_speculative`, `vil_sql_agent`

**Data / text / doc processing** — `vil_chunker`, `vil_data_prep`, `vil_doc_extract`, `vil_doc_layout`, `vil_doc_parser`, `vil_crawler`, `vil_tensor_shm`, `vil_vision`, `vil_audio`, `vil_output_parser`

**Built-in FaaS & utilities** — `vil_validate`, `vil_validate_derive`, `vil_validate_schema`, `vil_mask`, `vil_regex`, `vil_stats`, `vil_geodist`, `vil_phone`, `vil_email`, `vil_email_validate`, `vil_datefmt`, `vil_duration`, `vil_id_gen`, `vil_migrate`, `vil_anomaly`, `vil_ab_test`, `vil_bench_llm`, `vil_cost_tracker`, `vil_cache`, `vil_memory_graph`, `vil_context_optimizer`, `vil_script_js`, `vil_script_lua`, `vil_viz`, `vil_topo`, `vil_ir`, `vil_diag`, `vil_codegen_c`, `vil_codegen_rust`, `vil_connector_macros`, `vil_registry`, `vil_consensus`, `vil_edge`, `vil_edge_deploy`, `vil_macros`, `vil_lsp`

**CLI libraries** — `vil_cli_core`, `vil_cli_sdk`, `vil_cli_compile`, `vil_cli_pipeline`

> `vil_cli` (the `vil` binary dispatcher) is VSAL — see §3.2.

### 2.2 Permitted Uses (Unrestricted)

Under Apache 2.0 OR MIT, you may:

- Use VIL crates in any application (commercial or non-commercial) without restriction
- Modify VIL source code and distribute modified versions (with attribution)
- Build and sell commercial products, SaaS platforms, and services using VIL crates as dependencies
- Embed VIL crates in proprietary software without obligation to open-source your own code
- Build and operate any server, gateway, API, pipeline, or service using VIL crates as a framework
- Contribute improvements back to the VIL project (encouraged but not required)

### 2.3 Examples of Permitted Use

The following are explicitly permitted:

- Building an API gateway using `vil_new_http`, `vil_server_core`, and related crates, and selling it as a commercial product or SaaS
- Building an IoT platform using `vil_modbus`, `vil_opcua`, `vil_mq_mqtt`, and deploying it as a managed service
- Building a payment processing system using `vil_db_sqlx` and related crates
- Building a credit scoring engine using `vil_rules`, `vil_expr`, `vil_db_sqlx`, and operating it as a SaaS
- Building a data pipeline product using `vil_mq_kafka`, `vil_storage_s3`, and any VIL connectors
- Using VIL crates as part of a larger closed-source commercial application
- Forking VIL crates and creating derivative works for any purpose
- **Building any non-workflow SaaS product using VIL as a framework — fully unrestricted**

### 2.4 Attribution Requirements

As required by Apache 2.0 / MIT, you must:

- Include copies of the license(s) in distributions of VIL-derived software
- Preserve copyright notices in VIL source files
- State changes made to VIL source files in modified distributions

---

## 3. VIL Workflow & Server — Vastar Source Available License (VSAL)

### 3.1 Purpose

The VIL workflow and server runtime is source-available and free for most uses, but restricted against being repackaged as a Workflow-as-a-Service offering that competes with Vastar's commercial workflow services.

### 3.2 Scope of VSAL

VSAL applies to the following 7 crates — the actual WaaS vectors (workflow runtime + provisioning + operator + CLI):

| Crate | Role |
|-------|------|
| `vil_vwfd` | VWFD workflow compiler, executor, and `VwfdApp` builder (including `.provision()` API) |
| `vil_vwfd_macros` | Proc macros for compile-time VWFD workflow generation |
| `vil_server_provision` | Hot-reload provisioning engine — accepts workflow uploads at runtime (**primary WaaS vector**) |
| `vil_cli` | The `vil` binary — dispatcher for `init / dev / gen / deploy` (depends on `vil_cli_server` + `vil_vwfd`) |
| `vil_cli_server` | Backend for `vil dev / gen / deploy` (dev-loop server driver) |
| `vil_workflow_v2` | Next-generation DAG-based workflow scheduler (AI pipeline) |
| `vil_operator` | Kubernetes Operator (CRD-based deployment of vil-server) |

> **Note:** `vil_server` (the Axum-based server umbrella) remains Apache 2.0 / MIT — it is a convenience re-export of `vil_server_core` + sub-crates and does not itself host the WaaS vector. Any workflow-hosting product requires one of the six VSAL crates above.

These crates are source-available and **free to use for all purposes except building Workflow-as-a-Service as a primary product** (see Section 3.5). For building non-WaaS products (API gateways, IoT platforms, scoring engines, etc.), these crates are fully usable at no cost.

### 3.3 Definitions

#### 3.3.1 "Workflow-as-a-Service" (WaaS)

A product, platform, or service whose **primary purpose** is enabling third parties to create, deploy, provision, execute, or manage workflow definitions. This includes but is not limited to:

- Hosted workflow execution platforms
- Managed automation services
- General-purpose workflow orchestration platforms
- Any offering that competes with Vastar's commercial workflow services
- **Repackaging of third-party workflow formats** (n8n, Kestra, Airflow, Prefect, Dagster, Temporal, etc.) to run on VIL Server — the technical conversion does not change the WaaS classification

The technical implementation is irrelevant — whether using `.provision(true)`, building custom provisioning, converting other workflow DSLs to VWFD, or any other mechanism.

#### 3.3.2 "Significant Business Process"

A product delivers **substantial domain-specific value** beyond workflow execution. The workflow engine is an implementation detail, not the product's core value. Examples: credit scoring, payment processing, IoT monitoring, insurance underwriting, healthcare integration, e-commerce fulfillment, logistics routing.

#### 3.3.3 "Provisionable Mode"

Operation of `vil_server` / `vil_vwfd` with `.provision(true)` enabled, activating dynamic acceptance, registration, and execution of user-uploaded workflow definitions (VWFD YAML), WASM modules, Sidecar configurations, and native plugins (`.so`) via hot-reload provisioning.

#### 3.3.4 "Self-Use"

Deploying and operating VSAL-licensed components on infrastructure owned or controlled by the licensee, for the licensee's own internal purposes:

- Internal applications
- Internal automation
- Internal development
- Production workloads serving the licensee's own customers through applications built by the licensee

### 3.4 Grant of Rights

Subject to the terms of this License, Vastar grants you a worldwide, royalty-free, non-exclusive, non-transferable license to:

- Use, copy, modify, and distribute source of VSAL-covered crates
- Build and deploy `vil_server` in **Standard Server Mode** for any purpose, including commercial SaaS offerings (non-WaaS)
- Build and deploy `vil_server` in **Provisionable Mode for Self-Use**, including production workloads
- Create derivative works, subject to the same Workflow Service Restriction

### 3.5 Workflow Service Restriction

You may **NOT** use the VSAL workflow-runtime crates (`vil_vwfd`, `vil_vwfd_macros`, `vil_server_provision`, `vil_cli`, `vil_cli_server`, `vil_workflow_v2`, `vil_operator`, or any derivative) to build, operate, or offer Workflow-as-a-Service as a primary product. This restriction applies regardless of technical implementation — including when these crates are combined with `vil_server` (Apache/MIT) or any other component.

Specifically, you may **not**:

1. Operate a managed service whose primary purpose is allowing third parties to upload, deploy, execute, or manage workflow definitions (VWFD, YAML, JSON, or equivalent)
2. Build a platform-as-a-service (PaaS) or Workflow-as-a-Service (WaaS) where the core value proposition is workflow provisioning and execution
3. Re-package, re-brand, or white-label VIL's workflow engine capabilities as a hosted workflow platform
4. Offer "managed VIL", "hosted VIL workflows", or any equivalent service where third parties provision workflows to infrastructure you operate
5. Build a competing workflow orchestration cloud service using VIL's workflow crates as the engine
6. **Accept third-party workflow formats (n8n, Kestra, Airflow, Zapier, etc.) and translate them to VWFD to host on VIL Server as a managed service** — the translation does not exempt the service from WaaS classification

This restriction applies regardless of whether the service is offered for free or for a fee.

### 3.6 The Significant Business Process Exception

The Workflow Service Restriction does **NOT** apply when workflow provisioning is a component of a significant business process, not the primary product itself.

**The test is simple**:
> If you remove the workflow provisioning capability, does your product still have substantial value?
> - **Yes** → permitted
> - **No** → WaaS, not permitted

#### Examples: Significant Business Process (Permitted ✓)

- A **credit scoring platform** that uses VIL workflows internally to orchestrate scoring pipelines. The product is credit scoring, not workflow provisioning.
- An **IoT monitoring platform** where VIL workflows route sensor data and trigger alerts. The product is IoT monitoring.
- A **payment processing system** where VIL workflows manage transaction routing and fraud checks. The product is payment processing.
- An **insurance underwriting system** where VIL workflows orchestrate risk assessment. The product is underwriting.
- An **e-commerce platform** where VIL workflows manage order fulfillment, inventory, and notifications. The product is e-commerce.
- A **healthcare integration platform** where VIL workflows connect hospital systems. The product is healthcare integration.

#### Examples: WaaS (NOT Permitted ✗)

- A platform where users sign up and create/deploy their own arbitrary workflows — any workflow, any domain. **This is WaaS.**
- A "Serverless Workflow" platform where users upload YAML definitions and the platform executes them. **This is WaaS.**
- A "Managed VIL Cloud" service that provides workflow execution infrastructure. **This is WaaS.**
- An "automation platform" that competes with n8n/Kestra/Airflow using VIL as the engine. **This is WaaS.**
- A service that **accepts n8n (or Kestra, Airflow, Zapier, etc.) workflow files, converts them to VWFD, and hosts execution** on shared VIL Server infrastructure. **This is WaaS regardless of the conversion layer.**

### 3.7 Other Permitted Uses

#### 3.7.1 Building and Selling SaaS Products

You may build any commercial product or SaaS using VIL crates and `vil_server`. As long as your product delivers significant business value beyond workflow provisioning, workflow usage is permitted — even if your product internally uses `.provision(true)` for your own operational convenience.

#### 3.7.2 Self-Hosted (Internal Use)

You may deploy `vil_server` with `.provision(true)` on your own infrastructure for your own internal use. Your own team provisioning workflows to your own `vil_server` instances is Self-Use. This includes production workloads serving your own customers through applications you build.

#### 3.7.3 Client-Deployed Instances

If you build a product that includes `vil_server` and your clients deploy it on their own infrastructure, this is permitted. Each client's instance is Self-Use. You are distributing software, not operating a managed service.

#### 3.7.4 Open-Source Contribution

You may fork, modify, and contribute to VIL and `vil_server`. Derivative works carry the same Workflow Service Restriction.

#### 3.7.5 Licensor Reserved Rights (Vastar-Only Activities)

The Workflow Service Restriction in §3.5 applies to Licensees, **not to the Licensor**. Vastar (PT RAG Mid Solution) exclusively reserves the following commercial activities — these are Vastar's business model and are **not available** to third parties under VSAL:

1. **Managed PaaS / SaaS / WaaS** — Operating the VSAL crates (including `vil_vwfd`, `vil_server_provision`, `vil_workflow_v2`, `vil_operator`) as managed cloud services in Provisionable Mode, including multi-tenant workflow execution for third-party customers.

2. **AI-Powered Cloud Migration** — Accepting any source artifact (third-party workflow definitions such as n8n/Kestra/Temporal/Airflow/Prefect/Dagster/Zapier/ServiceNow/BPMN; specifications; Requests for Comments; design documents; legacy code; natural-language descriptions; or any other input) and translating, transforming, synthesizing, or compiling it into VWFD, VIL Projects, or equivalent format — whether by AI (ADE) or human-assisted — and hosting the resulting execution on Vastar infrastructure.

3. **Setup Project Services** — On-demand generation of VIL Projects from customer specifications, with Vastar-hosted execution as a deliverable.

4. **Commercial Sublicensing** — Licensing, sublicensing, white-labeling, or OEM embedding of the VSAL crates under separate negotiated commercial terms (for parties that need to offer WaaS legitimately).

These rights are codified in LICENSE-VSAL §5.2 (Licensor Reserved Rights) and §5.3 (Exclusive Commercial Operation). Third parties seeking any of these capabilities must negotiate a separate commercial agreement with Vastar (contact legal@vastar.id).

> **Why the asymmetry?** VSAL exists precisely to preserve these commercial channels as Vastar's moat. Without exclusivity, VIL could not sustain open-source development at its current scope (165+ Apache/MIT library crates). This is the same model as MongoDB SSPL, Elastic License v2, and BSL — the licensor operates the service; licensees build products.

> **Important — Licensor Reserved Rights do NOT narrow the Significant Business Process Exception (§3.6).** If VIL workflows are part of a Significant Business Process, you remain fully permitted to:
>
> - Use all 7 VSAL crates internally, including Provisionable Mode (`.provision(true)`), to orchestrate your own business logic
> - Expose your application to customers, even if customers interact with UI that triggers workflows underneath
> - Accept structured customer input (forms, CSV uploads, API payloads) that your predefined workflows process — this is data flow, not workflow provisioning
> - Ship your product to clients who deploy it themselves (software distribution, not managed service)
>
> The line is: **are customers uploading workflow definitions, or are they using your product?** The former is WaaS (licensee-forbidden); the latter is Significant Business Process (permitted).

### 3.8 Examples (Summary Table)

Legend: **SBPE** = Significant Business Process Exception (§3.6) — product delivers substantial domain value beyond workflow hosting.

| Scenario | Permitted? | Reasoning |
|----------|:----------:|-----------|
| Company A builds a credit scoring SaaS using VIL workflows to orchestrate scoring pipeline | ✓ | **SBPE.** Product = credit scoring. |
| Company B builds an IoT platform using VIL + Modbus/OPC-UA, internally uses `.provision(true)` | ✓ | **SBPE.** Product = IoT monitoring/control. |
| Company C deploys `vil_server` with `.provision(true)` internally for their own team | ✓ | Self-Use. |
| Company D builds a payment gateway using VIL, lets merchants configure payment routing rules | ✓ | **SBPE.** Routing rules = domain config, not arbitrary workflow. |
| Company E builds a product with `vil_server`. Their clients install it on-premise with `.provision(true)` | ✓ | Software distribution. Each client instance = Self-Use. |
| Company F builds a healthcare data integration platform using VIL workflows to connect hospital systems | ✓ | **SBPE.** Product = healthcare integration. |
| Company N builds an insurance underwriting SaaS — VIL workflows assess risk, score policies, route approvals | ✓ | **SBPE.** Product = underwriting. Customers submit applications, not workflows. |
| Company O builds an e-commerce order fulfillment platform — VIL orchestrates pick/pack/ship across warehouses | ✓ | **SBPE.** Product = fulfillment. |
| Company P builds a logistics/shipping platform — VIL workflows handle route optimization, customs clearance, tracking | ✓ | **SBPE.** Product = logistics. |
| Company Q builds a banking core — VIL workflows handle AML checks, transaction posting, reconciliation | ✓ | **SBPE.** Product = banking. Regulated domain logic. |
| Company R builds a KYC/compliance SaaS — VIL workflows run document verification, sanctions screening, PEP checks | ✓ | **SBPE.** Product = KYC. |
| Company S builds an HR/recruitment platform — VIL workflows orchestrate application screening, interview scheduling, onboarding | ✓ | **SBPE.** Product = HR tech. |
| Company T builds a manufacturing MES — VIL workflows coordinate machines, quality control, batch traceability | ✓ | **SBPE.** Product = manufacturing execution. |
| Company U builds a government e-service — VIL workflows route permit applications, approval chains, document verification | ✓ | **SBPE.** Product = e-government service. |
| Company V builds an LMS (learning platform) — VIL workflows handle enrollment, assessment, certificate issuance | ✓ | **SBPE.** Product = education. |
| Company W builds a SaaS that exposes a "workflow builder UI" to customers but customers configure **business rules** within the product's fixed domain model (e.g., credit policy thresholds) | ✓ | **SBPE.** Customers configure domain logic, not arbitrary workflows. |
| Company X builds a telehealth platform — VIL workflows orchestrate appointment booking, triage, prescription, pharmacy handoff | ✓ | **SBPE.** Product = telehealth. |
| Company Y builds an insurtech claims platform — internal team uses `.provision(true)` to iterate claim-handling workflows; customers submit claims | ✓ | **SBPE.** Provisioning is internal iteration; customers submit claims (data), not workflows. |
| Company G builds "WorkflowHub" — users sign up and deploy their own arbitrary YAML workflows to G's infrastructure | ✗ | WaaS. Primary product = workflow provisioning. |
| Company H builds "Automate.io" using VIL as engine — general-purpose automation competing with n8n/Zapier | ✗ | WaaS. Primary product = general workflow automation. |
| Company I builds custom provisioning layer using VIL crates (bypassing `.provision(true)`) to offer hosted workflow execution | ✗ | WaaS regardless of technical implementation. |
| Company J builds an n8n/Kestra compatibility layer that translates their workflows into VWFD and hosts on VIL Server as a managed service | ✗ | **WaaS. Format conversion does not exempt the service.** |
| AWS/GCP/Azure offers "Managed VIL Runtime" or "VIL Workflow Service" as a cloud product | ✗ | WaaS. Cloud provider workflow hosting is exclusively reserved for Vastar. |
| Company K builds "RFC → Workflow" service: customers submit PDFs/RFCs, Company K generates VIL projects and hosts execution | ✗ | WaaS + migration service. Both are Vastar-reserved (see §3.7.5). |
| Company L uses Vastar's AI-powered migration (ADE) to convert their Temporal workflows → VIL Project, then self-hosts on their own infra | ✓ | Licensee paid Vastar for migration; resulting VIL Project is Licensee's Self-Use. |
| **Vastar** offers "VIL Cloud Migration" — AI converts customer RFCs/n8n/Temporal/docs → VIL Projects, hosts on Vastar Cloud as PaaS/SaaS/WaaS | ✓ | **Licensor Reserved Right** (§3.7.5 / LICENSE-VSAL §5.2). |
| **Vastar** operates multi-tenant Provisionable Mode for third-party customers on Vastar Cloud | ✓ | **Licensor Reserved Right.** Same activity by any other party = ✗. |
| Company M contracts with Vastar for a commercial WaaS sublicense, then operates managed workflow hosting for their vertical (e.g., healthcare compliance) | ✓ | Permitted via separate commercial agreement with Vastar (§3.7.5 item 4). |

---

## 4. VFlow — Commercial License

### 4.1 Overview

VFlow is the high-performance commercial runtime, provided as a closed-source binary under a paid commercial license. VFlow includes all capabilities of `vil_server` plus:

- TriLaneKernel execution engine (significantly faster than VIL Server)
- Full V-CEL VM (lambda, regex, temporal types)
- VDICL bytecode rule engine
- 3-tier statestore (papaya + redb + WAL)
- Multi-tenancy, workflow versioning, blue-green deployment
- Rate limiting, policy enforcement
- Immediate durability, crash recovery

VFlow is **not free**. Because `vil_server` with `vil_vwfd` already provides a fully capable production runtime (hot-reload, WASM, Sidecar, native plugins, 30+ connectors, 13 triggers), VFlow is positioned as a premium upgrade for teams that need operational excellence at scale.

### 4.2 Permitted Uses

- Deploy VFlow on your own infrastructure for production workloads, subject to active license
- Use VFlow to serve your own customers through applications you build
- Deploy multiple VFlow instances within the scope of your license agreement

### 4.3 Restrictions

- You may **NOT** use VFlow without a valid commercial license from Vastar
- You may **NOT** offer VFlow as a managed service, hosted platform, or cloud offering to third parties
- You may **NOT** reverse-engineer, decompile, or disassemble VFlow
- You may **NOT** redistribute VFlow binaries to third parties outside the scope of your license
- You may **NOT** modify VFlow (source code is not available)
- The Workflow Service Restriction (Section 3.5) applies equally to VFlow

### 4.4 Pricing

VFlow is available under annual or multi-year license agreements. Contact [sales@vastar.id](mailto:sales@vastar.id) for pricing based on deployment scale, number of instances, and support requirements.

### 4.5 Managed VFlow Services

Managed VFlow services (VFlow Cloud, VFlow Enterprise Cloud) are offered exclusively by Vastar under separate service agreements. **No third party may operate managed VFlow services.**

---

## 5. VFlow Enterprise — Commercial License

VFlow Enterprise is licensed under a commercial agreement between Vastar and the licensee. Terms include:

- Annual or multi-year license fee
- Deployment options: Vastar-managed cloud OR customer on-premise
- High availability (HA) clustering support
- SSO / SCIM integration
- SLA with guaranteed uptime and response times
- Dedicated support channel
- Security certifications and compliance documentation

Contact [sales@vastar.id](mailto:sales@vastar.id) for commercial licensing inquiries.

---

## 6. Vastar Cloud Services

Vastar cloud services are governed by the Vastar Terms of Service and applicable service-level agreements. These are the **exclusive managed offerings for VIL/VFlow runtime**:

| Service | Tier | Description | Pricing |
|---------|------|-------------|---------|
| **VIL Cloud** | PaaS | Managed `vil_server` runtime — single-tenant deployment of VIL applications | Subscription ($5–30/month) |
| **VIL Cloud Workflow** | SaaS | Hosted VWFD runtime in Provisionable Mode — customer uploads workflows, Vastar hosts execution | Subscription / usage-based |
| **VIL Cloud WaaS** | WaaS | Multi-tenant managed Workflow-as-a-Service for third-party end-users (Vastar-exclusive per LICENSE-VSAL §5.3) | Usage-based / enterprise |
| **VIL Cloud Migration** | Migration-as-a-Service | AI-powered migration (ADE) — accepts n8n / Kestra / Airflow / Temporal / Prefect / Dagster / Zapier / ServiceNow / BPMN / RFCs / specifications / legacy code / design documents → outputs VIL Projects hosted on Vastar Cloud | Pay-per-project + hosting |
| **VIL Cloud Setup Project** | Professional Services + SaaS | On-demand VIL Project generation from customer specifications — deliverable is a provisionable VIL deployment hosted by Vastar | Fixed project fee + subscription |
| **VFlow Cloud** | PaaS | Managed VFlow runtime with hot-reload, multi-tenancy, versioning | Subscription ($40–100/month) |
| **VFlow Enterprise Cloud** | Managed / SLA | Vastar-managed enterprise deployment with HA, SSO, SLA | Custom agreement |
| **ADE** (Advanced Development Environment) | AI Service | AI-powered project setup (`vil init --advance`, `vil agent`) — standalone or feeds VIL Cloud Migration | Pay-per-use (Compilation Units) |
| **Galaxy Bimasakti** | Marketplace | Marketplace for templates, connectors, learning modules | 15% commission on paid items |

**Only Vastar may operate managed cloud services — including PaaS, SaaS, WaaS, and AI-powered Migration Services — that provide Provisionable Mode workflow execution.** This exclusivity is codified in LICENSE-VSAL §5.3 (Exclusive Commercial Operation) and protected by the VSAL applied to `vil_vwfd`, `vil_server_provision`, `vil_workflow_v2`, `vil_operator`, `vil_cli`, `vil_cli_server`, and `vil_vwfd_macros` (§3.2).

### 6.1 Vastar's AI-Powered Cloud Migration — The Commercial Moat

Vastar Cloud Migration is the **flagship commercial service** enabled by VSAL's §3.3.1 anti-translation clause. Third parties are forbidden from building "competitor-workflow → VWFD → hosted" translation pipelines (§3.5 + §3.8 Example J/K); only Vastar may operate this service.

**Accepted input formats** (non-exhaustive):

- **Workflow systems:** n8n, Kestra, Airflow, Prefect, Dagster, Temporal, Argo Workflows, Zapier, Make.com, ServiceNow Flow Designer, BPMN 2.0 XML, Camunda DMN, Appian, OutSystems
- **Specifications:** Requests for Comments (RFCs), system design documents, architecture diagrams, OpenAPI/Swagger specs, AsyncAPI, protobuf definitions
- **Legacy artifacts:** Legacy Java/Python/Go/COBOL business logic, stored procedures, mainframe JCL
- **Natural language:** English/Bahasa Indonesia prose descriptions of business processes
- **Hybrid:** Any combination of the above, with AI reconciliation

**Deliverable:** A provisionable VIL Project, deployed on Vastar Cloud (PaaS / SaaS / WaaS tier), with ongoing hosting and operational support.

**Why this is defensible:** The combination of (a) VSAL anti-translation restriction on licensees, (b) Vastar-exclusive Licensor Reserved Rights (§3.7.5, LICENSE-VSAL §5.2), and (c) AI-powered ADE tooling creates a durable commercial moat. Competitors cannot offer equivalent "bring your workflow, we host it" services on VIL infrastructure without a separate commercial agreement with Vastar.

---

## 7. Quick Reference

| Use Case | VIL Libraries (Apache/MIT) | VSAL Runtime | VFlow | Vastar-Operated |
|----------|:--------------------------:|:------------:|:-----:|:---------------:|
| Build any app/service using VIL as framework | ✓ Free | ✓ Free | N/A | N/A |
| Sell SaaS built with VIL (non-WaaS) | ✓ Free | ✓ Free | N/A | N/A |
| Self-host workflow engine (internal) | ✓ Free | ✓ Free | Paid license | N/A |
| Deploy to production (self-managed, non-WaaS) | ✓ Free | ✓ Free | Paid license | N/A |
| Build WaaS / general workflow platform | ✗ | ✗ Licensee | ✗ Licensee | ✓ Vastar-only |
| Offer managed workflow hosting to third parties | N/A | ✗ Licensee | ✗ Licensee | ✓ Vastar-only |
| Modify source code | ✓ Yes | ✓ Yes (VSAL) | ✗ No (closed) | N/A |
| Build SaaS with significant business process + workflow | ✓ Yes | ✓ Yes | Paid license | N/A |
| **Translate n8n/Kestra/Airflow/Temporal → VWFD and host as a service** | ✗ WaaS | ✗ Licensee | ✗ Licensee | ✓ **VIL Cloud Migration** |
| AI-powered migration from RFC/docs/specs → VIL Project + hosting | ✗ | ✗ Licensee | ✗ Licensee | ✓ **VIL Cloud Migration + Setup Project** |
| Multi-tenant Provisionable Mode WaaS | ✗ | ✗ Licensee | ✗ Licensee | ✓ **VIL Cloud WaaS** |

**Reading the table:** "Licensee" rows are activities forbidden to Licensees (third parties using VSAL crates). The "Vastar-Operated" column shows the same activities as Licensor Reserved Rights (§3.7.5, LICENSE-VSAL §5.2) — exclusively available through Vastar Cloud.

### One-Sentence Summary

> **Build anything with VIL. Deploy anywhere. Sell any SaaS.**
> **Just don't build a workflow platform — that's ours.**

---

## 8. General Terms

### 8.1 Intellectual Property

VIL, `vil_server`, VFlow, VFlow Enterprise, VScore, VIL IDE, VWFD, V-CEL, VDICL, VilQuery, TriLaneKernel, Galaxy Bimasakti, and Vastar are trademarks of **PT RAG Mid Solution**. Use of these trademarks is subject to Vastar's trademark guidelines ([TRADEMARK.md](docs/TRADEMARK.md)).

### 8.2 No Warranty

All software is provided "AS IS" without warranty of any kind, express or implied. Vastar shall not be liable for any damages arising from the use of the software.

### 8.3 Governing Law

This license is governed by the laws of the Republic of Indonesia. Any disputes shall be resolved in the courts of Jakarta, Indonesia.

### 8.4 License Updates

Vastar reserves the right to update the terms of the VSAL for future versions of VSAL-covered crates. Updated terms apply only to versions released after the update. Previously released versions remain under the terms in effect at the time of their release.

### 8.5 Contact

For licensing inquiries, commercial agreements, or clarification:

- **Email**: [legal@vastar.id](mailto:legal@vastar.id)
- **Sales**: [sales@vastar.id](mailto:sales@vastar.id)
- **Web**: [https://vastar.id/licensing](https://vastar.id/licensing)

---

**PT RAG Mid Solution**
*VIL Ecosystem Licensing Guide v1.0 — April 2026*
