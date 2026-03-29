#!/usr/bin/env bash
# =============================================================================
# VIL Batch Publish — 10 crates per run, 2 min between each
# =============================================================================
# Usage:
#   # Run once (publishes up to 10 new crates):
#   ./scripts/publish-batch.sh
#
#   # Loop until all done:
#   while ./scripts/publish-batch.sh; do echo "--- Waiting 10 min ---"; sleep 600; done
# =============================================================================

set -uo pipefail
cd "$(dirname "$0")/.."

BATCH=10
SKIP="vil_viz vil_script_js vil_script_lua"
PUBLISHED=0
RATE_LIMITED=0
FAILED=0
REMAINING=0

# ─────────────────────────────────────────────────────────────────────────────
# Dependency order — verified 2026-03-28
# Rule: every crate's VIL deps must appear ABOVE it in this list.
#
# KEY FIXES (observer integration):
#   - vil_observer bumped to 0.1.1 (added vil_log + connector_event deps)
#   - vil_server_core bumped to 0.1.1 (observer wiring)
#   - vil_new_http + vil_sdk BEFORE vil_server (was wrong before)
#   - vil_plugin_sdk BEFORE vil_server (only needs server_core)
# ─────────────────────────────────────────────────────────────────────────────
CRATES=(
    # L0: zero VIL deps
    vil_types vil_connector_macros vil_log vil_diag vil_topo vil_db_macros

    # L1: depends on vil_types only
    vil_json vil_shm vil_obs vil_ir vil_net

    # L2: depends on L0-1
    vil_queue vil_validate
    vil_registry  # shm + obs

    # L3: depends on L0-2
    vil_rt  # types+shm+queue+registry+obs
    vil_codegen_rust vil_codegen_c

    # L4: pre-server (NO server_core dep)
    vil_capsule   # types+ir
    vil_sidecar   # types+log
    vil_engine    # types+shm+queue+registry+rt

    # L5: observer (log + connector_macros only — NO server_core dep)
    vil_observer  # 0.1.1: added vil_log + vil_connector_macros

    # L6: server_core (capsule+sidecar+rt+obs+shm+json+log+observer)
    vil_server_core  # 0.1.1: observer wiring
    vil_server_config vil_server_format

    # L7: server sub-crates (need server_core)
    vil_server_web vil_server_mesh vil_server_auth vil_server_db
    vil_server_macros

    # L8: macros (needs server_core+ir+validate+types+json)
    vil_macros

    # L9: SDK + HTTP (must come BEFORE vil_server!)
    vil_new_http    # rt+ir+json — NO server dep
    vil_sdk         # engine+types+new_http+macros+ir+validate
    vil_plugin_sdk  # server_core only

    # L10: server umbrella
    vil_server      # server_core+server_web+mesh+auth+db+macros+sdk

    # L11: DB layer (before server_test and cache!)
    vil_db_redis    # server_core+server_db
    vil_db_semantic # server_db
    vil_db_sqlx vil_db_sea_orm

    # L12: server_test + cache (need DB crates above)
    vil_server_test # server + db_semantic
    vil_cache       # db_redis

    # L12: connectors + triggers + AI (need server_core or standalone)
    vil_mq_kafka vil_mq_mqtt vil_mq_nats
    vil_storage_s3 vil_storage_gcs vil_storage_azure
    vil_db_mongo vil_db_clickhouse vil_db_dynamodb
    vil_db_cassandra vil_db_timeseries vil_db_neo4j vil_db_elastic
    vil_mq_rabbitmq vil_mq_sqs vil_mq_pulsar vil_mq_pubsub
    vil_soap vil_opcua vil_modbus vil_ws
    vil_trigger_core vil_trigger_cron vil_trigger_fs
    vil_trigger_cdc vil_trigger_email vil_trigger_iot
    vil_trigger_evm vil_trigger_webhook
    vil_otel vil_edge_deploy

    # L13: AI/LLM stack
    vil_tokenizer vil_embedder vil_chunker vil_doc_parser
    vil_llm vil_rag vil_vectordb vil_reranker
    vil_llm_cache vil_llm_proxy vil_prompts vil_output_parser
    vil_guardrails vil_prompt_shield vil_ai_trace vil_cost_tracker
    vil_inference vil_model_serving vil_model_registry
    vil_quantized vil_speculative vil_tensor_shm
    vil_ai_gateway vil_semantic_router vil_context_optimizer
    vil_audio vil_vision vil_multimodal
    vil_doc_layout vil_doc_extract
    vil_crawler vil_index_updater
    vil_memory_graph vil_graphrag
    vil_realtime_rag vil_streaming_rag
    vil_federated_rag vil_private_rag
    vil_data_prep vil_synthetic vil_rlhf_data
    vil_bench_llm vil_eval vil_ab_test
    vil_feature_store vil_sql_agent
    vil_edge vil_consensus
    vil_workflow_v2
    vil_agent vil_multi_agent
    vil_ai_compiler vil_prompt_optimizer

    # L-extra: misc (some have server_core dep)
    vil_operator vil_lsp vil_grpc vil_graphql

    # L-last: viz + CLI (vil_viz is publish=false — skip via SKIP list)
    vil_viz vil_script_lua vil_script_js
    vil_cli
)

# ─────────────────────────────────────────────────────────────────────────────
# Helper: read version from crate's Cargo.toml
# ─────────────────────────────────────────────────────────────────────────────
get_local_version() {
    local crate="$1"
    local toml="crates/$crate/Cargo.toml"
    if [[ ! -f "$toml" ]]; then
        echo "0.0.0"
        return
    fi
    # Handle both `version = "x.y.z"` and `version.workspace = true`
    local ver
    ver=$(grep '^version' "$toml" | head -1)
    if echo "$ver" | grep -q 'workspace'; then
        # Read workspace version from root Cargo.toml
        ver=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
    else
        ver=$(echo "$ver" | sed 's/.*"\(.*\)"/\1/')
    fi
    echo "$ver"
}

# Deduplicate
declare -A SEEN
UNIQUE=()
for c in "${CRATES[@]}"; do
    if [[ -z "${SEEN[$c]:-}" ]]; then
        SEEN[$c]=1
        UNIQUE+=("$c")
    fi
done

echo "=== VIL Batch Publish — $(date) ==="

for crate in "${UNIQUE[@]}"; do
    # Skip list
    for skip in $SKIP; do
        [[ "$crate" == "$skip" ]] && continue 2
    done

    # Read local version and check crates.io
    local_ver=$(get_local_version "$crate")
    status=$(curl -s -o /dev/null -w "%{http_code}" "https://crates.io/api/v1/crates/$crate/$local_ver")
    if [[ "$status" == "200" ]]; then
        continue  # already published at this version
    fi

    REMAINING=$((REMAINING + 1))

    # Publish (with 1 retry on transient failures)
    echo -n "  Publishing $crate v$local_ver ... "
    attempt=0
    published=false
    while [[ $attempt -lt 2 ]]; do
        output=$(cargo publish -p "$crate" --allow-dirty 2>&1)
        if echo "$output" | grep -q "Published\|Uploaded"; then
            echo "✅"
            PUBLISHED=$((PUBLISHED + 1))
            published=true
            break
        elif echo "$output" | grep -q "already exists"; then
            echo "⏭ (exists)"
            published=true
            break
        elif echo "$output" | grep -q "429\|Too Many Requests"; then
            echo "⏳ rate limited — stopping batch"
            RATE_LIMITED=1
            break 2  # exit outer for loop
        elif echo "$output" | grep -q "503\|x-timer\|timed out\|Service Unavailable"; then
            # Transient crates.io error — retry once after 30s
            attempt=$((attempt + 1))
            if [[ $attempt -lt 2 ]]; then
                echo -n "⚠ transient error, retry in 30s ... "
                sleep 30
            fi
        else
            break  # real error, don't retry
        fi
    done

    if [[ "$published" != "true" && $RATE_LIMITED -eq 0 ]]; then
        echo "❌"
        echo "$output" | grep -v "^$" | tail -4 | sed 's/^/    /'
        FAILED=$((FAILED + 1))
    fi

    if [[ $PUBLISHED -ge $BATCH ]]; then
        echo "  Batch of $BATCH done."
        break
    fi

    # 10.000-15.999s random delay (millisecond precision)
    delay_s=$((RANDOM % 6 + 10))
    delay_ms=$((RANDOM % 1000))
    sleep "${delay_s}.$(printf '%03d' $delay_ms)"
done

echo ""
echo "Published this batch: $PUBLISHED | Failed: $FAILED | Rate limited: $RATE_LIMITED"

if [[ $REMAINING -eq 0 && $RATE_LIMITED -eq 0 ]]; then
    echo "=== All crates published! ==="
    exit 1
elif [[ $PUBLISHED -gt 0 || $RATE_LIMITED -gt 0 ]]; then
    echo "=== More to do — will retry after cooldown ==="
    exit 0
else
    echo "=== Some crates failed (dependency order?) — will retry ==="
    exit 0
fi
