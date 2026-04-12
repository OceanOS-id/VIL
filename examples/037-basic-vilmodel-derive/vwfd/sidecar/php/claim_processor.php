#!/usr/bin/env php
<?php
// Insurance Claim Processor — PHP Sidecar (VIL SDK UDS+SHM with stdin/stdout fallback)

function processClaim($input) {
    $claim_id = 'CLM-' . strtoupper(substr(md5(microtime()), 0, 8));
    $amount = $input['amount'] ?? 0;
    $type = $input['claim_type'] ?? 'general';
    $status = $amount > 50000000 ? 'manual_review' : 'auto_approved';
    $payout = $status === 'auto_approved' ? $amount * 0.8 : 0;
    return [
        'claim_id' => $claim_id, 'type' => $type, 'amount' => $amount,
        'status' => $status, 'estimated_payout' => $payout, 'currency' => 'IDR'
    ];
}

if (getenv('VIL_SIDECAR_SOCKET')) {
    // UDS+SHM mode
    require_once __DIR__ . '/../../../../crates/vil_sidecar/sdk/vil_sidecar_sdk.php';
    $app = new VilSidecarApp('claim_processor');
    $app->handler('execute', 'processClaim');
    $app->run();
} else {
    // Stdin/stdout line-delimited JSON (fallback)
    while (($line = fgets(STDIN)) !== false) {
        $line = trim($line);
        if ($line === '') continue;
        $input = json_decode($line, true) ?: [];
        echo json_encode(processClaim($input)) . "\n";
        flush();
    }
}
