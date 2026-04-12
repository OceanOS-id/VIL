#!/usr/bin/env node
/**
 * VIL Sidecar SDK — Node.js
 * Connect to VIL host via UDS, exchange data via SHM, handle Invoke/Result.
 *
 * Usage:
 *   const { SidecarApp } = require('./vil_sidecar_sdk');
 *   const app = new SidecarApp('my-scorer');
 *   app.handler('predict', (data) => ({ score: 0.95 }));
 *   app.run();
 */
const net = require('net');
const fs = require('fs');

class SidecarApp {
    constructor(name, version = '1.0.0') {
        this.name = name;
        this.version = version;
        this.handlers = {};
        this.conn = null;
        this.shmFd = null;
        this.shmBuf = null;
        this.recvBuf = Buffer.alloc(0);
    }

    handler(method, fn) {
        this.handlers[method] = fn;
        return this;
    }

    run() {
        const socketPath = process.env.VIL_SIDECAR_SOCKET ||
            `/tmp/vil_sidecar_${this.name}.sock`;

        this.conn = net.createConnection(socketPath, () => {
            this._send({
                type: 'Handshake', name: this.name, version: this.version,
                methods: Object.keys(this.handlers), capabilities: [], auth_token: null,
            });
        });

        this.conn.on('data', (chunk) => {
            this.recvBuf = Buffer.concat([this.recvBuf, chunk]);
            this._processMessages();
        });

        this.conn.on('error', () => process.exit(0));
        this.conn.on('end', () => process.exit(0));
    }

    _processMessages() {
        while (this.recvBuf.length >= 4) {
            const len = this.recvBuf.readUInt32LE(0);
            if (this.recvBuf.length < 4 + len) break;
            const payload = this.recvBuf.slice(4, 4 + len);
            this.recvBuf = this.recvBuf.slice(4 + len);
            const msg = JSON.parse(payload);
            this._dispatch(msg);
        }
    }

    _dispatch(msg) {
        if (msg.type === 'HandshakeAck') {
            if (!msg.accepted) { console.error('Handshake rejected:', msg.reject_reason); process.exit(1); }
            try {
                this.shmFd = fs.openSync(msg.shm_path, 'r+');
                this.shmBuf = Buffer.alloc(msg.shm_size);
                // mmap not natively available in Node — use fd read/write
            } catch (e) { /* SHM optional for small payloads */ }
        } else if (msg.type === 'Invoke') {
            this._handleInvoke(msg);
        } else if (msg.type === 'Health') {
            this._send({ type: 'HealthOk', in_flight: 0, total_processed: 0, total_errors: 0, uptime_secs: 0 });
        } else if (msg.type === 'Drain') {
            this._send({ type: 'Drained' });
        } else if (msg.type === 'Shutdown') {
            process.exit(0);
        }
    }

    _handleInvoke(msg) {
        const { descriptor, method } = msg;
        const requestId = descriptor.request_id;

        // Read from SHM via fd
        let inputData = {};
        if (this.shmFd !== null && descriptor.len > 0) {
            const buf = Buffer.alloc(descriptor.len);
            fs.readSync(this.shmFd, buf, 0, descriptor.len, descriptor.offset);
            try { inputData = JSON.parse(buf.toString()); } catch {}
        }

        const handler = this.handlers[method];
        if (!handler) {
            this._sendResult(requestId, 'MethodNotFound', 0, 0, `no handler for '${method}'`);
            return;
        }

        try {
            const result = handler(inputData);
            const resultBuf = Buffer.from(JSON.stringify(result));
            const respOffset = 1024 * 1024;
            if (this.shmFd !== null) {
                fs.writeSync(this.shmFd, resultBuf, 0, resultBuf.length, respOffset);
            }
            this._sendResult(requestId, 'Ok', respOffset, resultBuf.length);
        } catch (e) {
            this._sendResult(requestId, 'Error', 0, 0, e.message);
        }
    }

    _sendResult(requestId, status, offset = 0, len = 0, error = null) {
        this._send({
            type: 'Result', request_id: requestId, status,
            descriptor: status === 'Ok' ? {
                request_id: requestId, region_id: 0, _pad0: 0,
                offset, len, method_hash: 0, timeout_ms: 0, flags: 0,
            } : null,
            error,
        });
    }

    _send(msg) {
        const data = Buffer.from(JSON.stringify(msg));
        const header = Buffer.alloc(4);
        header.writeUInt32LE(data.length);
        this.conn.write(Buffer.concat([header, data]));
    }
}

module.exports = { SidecarApp };
