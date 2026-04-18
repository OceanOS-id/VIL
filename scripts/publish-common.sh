#!/usr/bin/env bash
# =============================================================================
# Shared crate list + skip list + version check
# Source this file from publish-all.sh and publish-batch.sh.
# Single source of truth for what crates get published and in what order.
# =============================================================================

# ── VSAL + legacy crates that must NEVER be published to crates.io ──────────
# vil_script_js, vil_script_lua — publish = false (legacy scripting)
# VSAL crates (v0.4.0+) — source-available, NOT on crates.io:
#   vil_cli, vil_cli_server, vil_vwfd, vil_vwfd_macros,
#   vil_workflow_v2, vil_operator, vil_server_provision
export PUBLISH_SKIP_CRATES="vil_script_js vil_script_lua vil_cli vil_cli_server vil_vwfd vil_vwfd_macros vil_workflow_v2 vil_operator vil_server_provision vil-server-provision"

# ── Crate list in dependency-safe publish order (Apache/MIT only) ───────────
# Rule: every crate's VIL deps must appear ABOVE it in this list.
# When adding a new crate: find its deepest vil_* dep, insert BELOW that.
export PUBLISH_CRATES=(
    # ── L0 — leaf deps, no VIL-internal deps ────────────────────────────
    vil_types vil_connector_macros vil_json vil_log vil_diag vil_topo
    vil_db_macros vil_grpc vil_server_format vil_server_config
    vil_shm vil_obs vil_ir vil_sidecar
    vil_queue vil_net vil_validate vil_validate_derive
    vil_capsule vil_codegen_rust vil_codegen_c
    vil_migrate

    # ── L1 — single-level deps on L0 ────────────────────────────────────
    vil_registry
    vil_rt
    vil_engine vil_new_http
    vil_observer vil_lsp
    vil_server_core
    vil_server_web vil_server_mesh vil_server_auth vil_server_db
    vil_server_macros vil_plugin_sdk
    vil_macros vil_db_semantic vil_db_sqlx vil_db_sea_orm vil_db_redis
    vil_sdk vil_cache vil_graphql
    vil_server
    vil_server_test

    # ── L2 — connectors + triggers + misc ──────────────────────────────
    vil_storage_s3 vil_storage_gcs vil_storage_azure
    vil_db_mongo vil_db_clickhouse vil_db_dynamodb vil_db_cassandra
    vil_db_timeseries vil_db_neo4j vil_db_elastic
    vil_mq_kafka vil_mq_mqtt vil_mq_nats vil_mq_rabbitmq vil_mq_sqs
    vil_mq_pulsar vil_mq_pubsub
    vil_soap vil_opcua vil_modbus vil_ws
    vil_trigger_core vil_trigger_cron vil_trigger_fs vil_trigger_cdc
    vil_trigger_email vil_trigger_iot vil_trigger_evm vil_trigger_webhook
    vil_trigger_kafka vil_trigger_s3 vil_trigger_sftp vil_trigger_db_poll vil_trigger_grpc
    vil_hash vil_crypto vil_jwt vil_id_gen
    vil_datefmt vil_duration
    vil_parse_csv vil_parse_xml
    vil_regex vil_phone
    vil_validate_schema vil_mask vil_reshape
    vil_template vil_email_validate
    vil_stats vil_anomaly
    vil_email vil_webhook_out vil_geodist
    vil_orm_derive vil_orm
    vil_expr
    vil_rules
    vil_cli_core vil_cli_compile vil_cli_sdk vil_cli_pipeline
    vil_otel vil_edge_deploy

    # ── L3 — AI / LLM / RAG stack ──────────────────────────────────────
    vil_tokenizer vil_embedder vil_llm vil_vectordb vil_inference
    vil_tensor_shm vil_ai_compiler vil_semantic_router vil_prompt_shield
    vil_quantized vil_memory_graph vil_feature_store vil_doc_parser
    vil_doc_layout vil_prompts vil_doc_extract vil_crawler vil_audio
    vil_vision vil_sql_agent vil_ab_test vil_cost_tracker
    vil_model_registry vil_data_prep vil_synthetic vil_rlhf_data
    vil_bench_llm vil_index_updater vil_federated_rag vil_private_rag
    vil_edge vil_output_parser vil_ai_trace vil_prompt_optimizer
    vil_context_optimizer vil_chunker vil_streaming_rag vil_realtime_rag
    vil_llm_cache vil_multimodal vil_llm_proxy vil_speculative
    vil_ai_gateway vil_model_serving vil_consensus vil_eval
    vil_guardrails vil_graphrag
    vil_rag vil_reranker
    vil_agent
    vil_multi_agent

    # ── L4 — viz + umbrella ─────────────────────────────────────────────
    vil_viz
    vil
)

# ── Version preflight ───────────────────────────────────────────────────────
# Usage: `publish_preflight_version [expected_version]`
# Expected defaults to 0.4.0; override via VIL_PUBLISH_VERSION env.
publish_preflight_version() {
    local expected="${1:-${VIL_PUBLISH_VERSION:-0.4.0}}"
    local actual
    actual=$(awk '/^\[workspace\.package\]/{f=1} f && /^version = /{gsub(/"|version = /,""); print; exit}' Cargo.toml)
    if [[ "$actual" != "$expected" ]]; then
        echo "✗ Workspace version mismatch:"
        echo "    Cargo.toml [workspace.package] version = \"$actual\""
        echo "    Expected                                = \"$expected\""
        echo "  Override: VIL_PUBLISH_VERSION=$actual $0"
        return 1
    fi
    echo "✓ Workspace version = $actual"
    return 0
}

# ── Deduplicate helper ──────────────────────────────────────────────────────
# Ensures the CRATES list has no duplicates even if accidentally introduced.
publish_dedupe_crates() {
    declare -A seen
    local unique=()
    local c
    for c in "${PUBLISH_CRATES[@]}"; do
        if [[ -z "${seen[$c]+_}" ]]; then
            seen[$c]=1
            unique+=("$c")
        fi
    done
    printf '%s\n' "${unique[@]}"
}
