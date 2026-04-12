#!/usr/bin/env python3
"""
VIL Sidecar SDK — Python
Connect to VIL host via UDS, exchange data via SHM, handle Invoke/Result protocol.

Usage:
    from vil_sidecar_sdk import SidecarApp
    app = SidecarApp("my-scorer")

    @app.handler("predict")
    def predict(data: dict) -> dict:
        return {"score": 0.95}

    app.run()
"""
import json, struct, socket, mmap, os, sys, time, traceback

class SidecarApp:
    def __init__(self, name: str, version: str = "1.0.0"):
        self.name = name
        self.version = version
        self.handlers = {}
        self.conn = None
        self.shm_fd = None
        self.shm_mm = None
        self.shm_size = 0

    def handler(self, method: str):
        """Decorator to register a handler for a method name."""
        def decorator(fn):
            self.handlers[method] = fn
            return fn
        return decorator

    def run(self):
        """Connect to VIL host, perform handshake, serve requests."""
        socket_path = os.environ.get("VIL_SIDECAR_SOCKET",
                                      f"/tmp/vil_sidecar_{self.name}.sock")
        self.conn = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.conn.connect(socket_path)

        # Send Handshake
        handshake = {
            "type": "Handshake",
            "name": self.name,
            "version": self.version,
            "methods": list(self.handlers.keys()),
            "capabilities": [],
            "auth_token": None,
        }
        self._send(handshake)

        # Receive HandshakeAck
        ack = self._recv()
        if ack.get("type") != "HandshakeAck" or not ack.get("accepted"):
            print(f"[VIL Sidecar] Handshake rejected: {ack.get('reject_reason')}", file=sys.stderr)
            return

        # Setup SHM
        shm_path = ack["shm_path"]
        self.shm_size = ack["shm_size"]
        self.shm_fd = os.open(shm_path, os.O_RDWR)
        self.shm_mm = mmap.mmap(self.shm_fd, self.shm_size)

        # Main loop
        while True:
            try:
                msg = self._recv()
            except (ConnectionError, struct.error):
                break

            msg_type = msg.get("type")
            if msg_type == "Invoke":
                self._handle_invoke(msg)
            elif msg_type == "Health":
                self._send({"type": "HealthOk", "in_flight": 0,
                            "total_processed": 0, "total_errors": 0,
                            "uptime_secs": 0})
            elif msg_type == "Drain":
                self._send({"type": "Drained"})
            elif msg_type == "Shutdown":
                break

        self._cleanup()

    def _handle_invoke(self, msg):
        desc = msg["descriptor"]
        method = msg["method"]
        request_id = desc["request_id"]
        offset = desc["offset"]
        length = desc["len"]

        # Read request data from SHM
        self.shm_mm.seek(offset)
        raw = self.shm_mm.read(length)
        try:
            input_data = json.loads(raw)
        except json.JSONDecodeError:
            input_data = raw.decode("utf-8", errors="replace")

        # Dispatch to handler
        handler = self.handlers.get(method)
        if handler is None:
            self._send_result(request_id, "MethodNotFound", error=f"no handler for '{method}'")
            return

        try:
            result = handler(input_data)
            result_bytes = json.dumps(result).encode("utf-8")

            # Write response to SHM (after request region)
            resp_offset = 1024 * 1024  # 1MB offset for responses
            self.shm_mm.seek(resp_offset)
            self.shm_mm.write(result_bytes)

            self._send_result(request_id, "Ok", resp_offset, len(result_bytes))
        except Exception as e:
            self._send_result(request_id, "Error", error=str(e))

    def _send_result(self, request_id, status, offset=0, length=0, error=None):
        msg = {
            "type": "Result",
            "request_id": request_id,
            "status": status,
            "descriptor": {"request_id": request_id, "region_id": 0, "_pad0": 0,
                           "offset": offset, "len": length,
                           "method_hash": 0, "timeout_ms": 0, "flags": 0} if status == "Ok" else None,
            "error": error,
        }
        self._send(msg)

    def _send(self, msg):
        data = json.dumps(msg).encode("utf-8")
        self.conn.sendall(struct.pack("<I", len(data)) + data)

    def _recv(self):
        raw_len = self._recv_exact(4)
        length = struct.unpack("<I", raw_len)[0]
        raw_data = self._recv_exact(length)
        return json.loads(raw_data)

    def _recv_exact(self, n):
        buf = b""
        while len(buf) < n:
            chunk = self.conn.recv(n - len(buf))
            if not chunk:
                raise ConnectionError("connection closed")
            buf += chunk
        return buf

    def _cleanup(self):
        if self.shm_mm:
            self.shm_mm.close()
        if self.shm_fd is not None:
            os.close(self.shm_fd)
        if self.conn:
            self.conn.close()
