#!/usr/bin/env php
<?php
/**
 * VIL Sidecar SDK — PHP
 * Connect to VIL host via UDS, exchange data via SHM, handle Invoke/Result.
 *
 * Usage:
 *   require_once 'vil_sidecar_sdk.php';
 *   $app = new VilSidecarApp('my-scorer');
 *   $app->handler('predict', function($data) { return ['score' => 0.95]; });
 *   $app->run();
 */
class VilSidecarApp {
    private string $name;
    private string $version;
    private array $handlers = [];
    private $conn;
    private $shmId = null;
    private int $shmSize = 0;

    public function __construct(string $name, string $version = '1.0.0') {
        $this->name = $name;
        $this->version = $version;
    }

    public function handler(string $method, callable $fn): self {
        $this->handlers[$method] = $fn;
        return $this;
    }

    public function run(): void {
        $socketPath = getenv('VIL_SIDECAR_SOCKET') ?: "/tmp/vil_sidecar_{$this->name}.sock";
        $this->conn = socket_create(AF_UNIX, SOCK_STREAM, 0);
        if (!socket_connect($this->conn, $socketPath)) {
            fwrite(STDERR, "[VIL Sidecar] Failed to connect to $socketPath\n");
            return;
        }

        // Send Handshake
        $this->send([
            'type' => 'Handshake', 'name' => $this->name, 'version' => $this->version,
            'methods' => array_keys($this->handlers), 'capabilities' => [], 'auth_token' => null,
        ]);

        // Receive HandshakeAck
        $ack = $this->recv();
        if (($ack['type'] ?? '') !== 'HandshakeAck' || !($ack['accepted'] ?? false)) {
            fwrite(STDERR, "[VIL Sidecar] Handshake rejected: " . ($ack['reject_reason'] ?? 'unknown') . "\n");
            return;
        }

        // Setup SHM
        $shmPath = $ack['shm_path'] ?? '';
        $this->shmSize = (int)($ack['shm_size'] ?? 0);
        if ($shmPath && file_exists($shmPath)) {
            $fd = fopen($shmPath, 'r+b');
            if ($fd) $this->shmId = $fd;
        }

        // Main loop
        while (true) {
            try {
                $msg = $this->recv();
            } catch (\Throwable $e) {
                break;
            }

            switch ($msg['type'] ?? '') {
                case 'Invoke': $this->handleInvoke($msg); break;
                case 'Health':
                    $this->send(['type' => 'HealthOk', 'in_flight' => 0,
                                 'total_processed' => 0, 'total_errors' => 0, 'uptime_secs' => 0]);
                    break;
                case 'Drain': $this->send(['type' => 'Drained']); break;
                case 'Shutdown': break 2;
            }
        }

        $this->cleanup();
    }

    private function handleInvoke(array $msg): void {
        $desc = $msg['descriptor'];
        $method = $msg['method'];
        $requestId = $desc['request_id'];

        // Read from SHM
        $inputData = [];
        if ($this->shmId && $desc['len'] > 0) {
            fseek($this->shmId, $desc['offset']);
            $raw = fread($this->shmId, $desc['len']);
            $inputData = json_decode($raw, true) ?: [];
        }

        $handler = $this->handlers[$method] ?? null;
        if (!$handler) {
            $this->sendResult($requestId, 'MethodNotFound', 0, 0, "no handler for '$method'");
            return;
        }

        try {
            $result = $handler($inputData);
            $resultBytes = json_encode($result);
            $respOffset = 1024 * 1024;
            if ($this->shmId) {
                fseek($this->shmId, $respOffset);
                fwrite($this->shmId, $resultBytes);
            }
            $this->sendResult($requestId, 'Ok', $respOffset, strlen($resultBytes));
        } catch (\Throwable $e) {
            $this->sendResult($requestId, 'Error', 0, 0, $e->getMessage());
        }
    }

    private function sendResult(int $requestId, string $status, int $offset = 0, int $len = 0, ?string $error = null): void {
        $this->send([
            'type' => 'Result', 'request_id' => $requestId, 'status' => $status,
            'descriptor' => $status === 'Ok' ? [
                'request_id' => $requestId, 'region_id' => 0, '_pad0' => 0,
                'offset' => $offset, 'len' => $len, 'method_hash' => 0, 'timeout_ms' => 0, 'flags' => 0,
            ] : null,
            'error' => $error,
        ]);
    }

    private function send(array $msg): void {
        $data = json_encode($msg);
        $header = pack('V', strlen($data));
        socket_write($this->conn, $header . $data);
    }

    private function recv(): array {
        $header = $this->recvExact(4);
        $len = unpack('V', $header)[1];
        $data = $this->recvExact($len);
        return json_decode($data, true) ?: [];
    }

    private function recvExact(int $n): string {
        $buf = '';
        while (strlen($buf) < $n) {
            $chunk = socket_read($this->conn, $n - strlen($buf));
            if ($chunk === false || $chunk === '') throw new \RuntimeException('connection closed');
            $buf .= $chunk;
        }
        return $buf;
    }

    private function cleanup(): void {
        if ($this->shmId) fclose($this->shmId);
        if ($this->conn) socket_close($this->conn);
    }
}
