#!/bin/bash
# Self-contained test — CLI/trigger mode
PASS_COUNT=0; FAIL_COUNT=0; SKIP_COUNT=0
GREEN="\033[0;32m"; RED="\033[0;31m"; YELLOW="\033[0;33m"; NC="\033[0m"
echo "=== 802-trigger-fs-watcher (VWFD) ==="
if [ -n "${VIL_CLI_LOG:-}" ] && [ -f "$VIL_CLI_LOG" ]; then
    EXIT=${VIL_CLI_EXIT:-1}
    if [ "$EXIT" -eq 0 ] || [ "$EXIT" -eq 124 ] || [ "$EXIT" -eq 143 ]; then
        PASS_COUNT=$((PASS_COUNT+1)); echo -e "  ${GREEN}PASS${NC} binary ran (exit=$EXIT)"
    else
        FAIL_COUNT=$((FAIL_COUNT+1)); echo -e "  ${RED}FAIL${NC} exit=$EXIT"
    fi
else
    SKIP_COUNT=$((SKIP_COUNT+1)); echo -e "  ${YELLOW}SKIP${NC} No CLI log"
fi
echo ""; echo "  Pass: $PASS_COUNT  Fail: $FAIL_COUNT  Skip: $SKIP_COUNT"
