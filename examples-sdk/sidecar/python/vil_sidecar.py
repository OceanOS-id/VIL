"""
VIL Sidecar SDK for Python
==============================

Implements the VIL sidecar protocol:
  - UDS transport with length-prefixed JSON framing
  - Handshake/HandshakeAck
  - Invoke/Result pattern
  - Health/Drain/Shutdown lifecycle

Usage:
    from vil_sidecar import VlangSidecar

    sidecar = VlangSidecar("fraud-detector")

    @sidecar.method("score")
    def score_transaction(data: dict) -> dict:
        return {"risk_score": 0.85, "action": "flag"}

    sidecar.run()
"""

import asyncio
import json
import struct
import os
import time
import signal
from typing import Callable, Dict, Optional


class VlangSidecar:
    """VIL Sidecar process -- implements the host-facing protocol."""

    def __init__(self, name: str, version: str = "1.0"):
        self.name = name
        self.version = version
        self._methods: Dict[str, Callable] = {}
        self._total_processed = 0
        self._total_errors = 0
        self._in_flight = 0
        self._start_time = time.time()
        self._draining = False
        self._socket_path = f"/tmp/vil_sidecar_{name}.sock"

    def method(self, name: str):
        """Decorator to register a sidecar method handler."""
        def decorator(func: Callable[[dict], dict]):
            self._methods[name] = func
            return func
        return decorator

    def run(self, socket_path: Optional[str] = None):
        """Start the sidecar and listen for connections."""
        if socket_path:
            self._socket_path = socket_path

        print(f"[vil-sidecar] {self.name} v{self.version}")
        print(f"[vil-sidecar] Methods: {list(self._methods.keys())}")
        print(f"[vil-sidecar] Listening on {self._socket_path}")

        loop = asyncio.new_event_loop()
        for sig in (signal.SIGINT, signal.SIGTERM):
            loop.add_signal_handler(sig, lambda: asyncio.ensure_future(self._shutdown(loop)))

        try:
            loop.run_until_complete(self._serve())
        except KeyboardInterrupt:
            pass
        finally:
            loop.close()

    async def _serve(self):
        try:
            os.unlink(self._socket_path)
        except FileNotFoundError:
            pass

        server = await asyncio.start_unix_server(
            self._handle_connection, path=self._socket_path
        )
        async with server:
            await server.serve_forever()

    async def _handle_connection(self, reader: asyncio.StreamReader, writer: asyncio.StreamWriter):
        print("[vil-sidecar] Connection accepted")
        try:
            while True:
                msg = await self._recv(reader)
                if msg is None:
                    break

                msg_type = msg.get("type")

                if msg_type == "Handshake":
                    ack = {
                        "type": "HandshakeAck",
                        "accepted": True,
                        "shm_path": f"/dev/shm/vil_sc_{self.name}",
                        "shm_size": 67108864,
                        "reject_reason": None,
                    }
                    await self._send(writer, ack)
                    print("[vil-sidecar] Handshake completed")

                elif msg_type == "Invoke":
                    await self._handle_invoke(writer, msg)

                elif msg_type == "Health":
                    await self._send(writer, {
                        "type": "HealthOk",
                        "in_flight": self._in_flight,
                        "total_processed": self._total_processed,
                        "total_errors": self._total_errors,
                        "uptime_secs": int(time.time() - self._start_time),
                    })

                elif msg_type == "Drain":
                    self._draining = True
                    while self._in_flight > 0:
                        await asyncio.sleep(0.1)
                    await self._send(writer, {"type": "Drained"})
                    print("[vil-sidecar] Drained")

                elif msg_type == "Shutdown":
                    print("[vil-sidecar] Shutdown requested")
                    break

        except (ConnectionError, asyncio.IncompleteReadError):
            print("[vil-sidecar] Connection closed")
        finally:
            writer.close()

    async def _handle_invoke(self, writer, msg: dict):
        method_name = msg.get("method", "")
        descriptor = msg.get("descriptor", {})
        request_id = descriptor.get("request_id", 0)

        self._in_flight += 1
        self._total_processed += 1

        handler = self._methods.get(method_name)
        if handler is None:
            self._in_flight -= 1
            self._total_errors += 1
            await self._send(writer, {
                "type": "Result",
                "request_id": request_id,
                "status": "MethodNotFound",
                "descriptor": None,
                "error": f"method '{method_name}' not registered",
            })
            return

        try:
            input_data = {
                "method": method_name,
                "request_id": request_id,
                "offset": descriptor.get("offset", 0),
                "len": descriptor.get("len", 0),
            }
            output = handler(input_data)
            output_json = json.dumps(output)

            await self._send(writer, {
                "type": "Result",
                "request_id": request_id,
                "status": "Ok",
                "descriptor": {
                    "request_id": request_id,
                    "slot": 0,
                    "offset": 0,
                    "len": len(output_json),
                    "method": None,
                    "timeout_ms": None,
                },
                "error": None,
            })
        except Exception as e:
            self._total_errors += 1
            await self._send(writer, {
                "type": "Result",
                "request_id": request_id,
                "status": "Error",
                "descriptor": None,
                "error": str(e),
            })
        finally:
            self._in_flight -= 1

    async def _recv(self, reader: asyncio.StreamReader) -> Optional[dict]:
        try:
            len_bytes = await reader.readexactly(4)
            length = struct.unpack("<I", len_bytes)[0]
            if length > 16 * 1024 * 1024:
                return None
            payload = await reader.readexactly(length)
            return json.loads(payload)
        except asyncio.IncompleteReadError:
            return None

    async def _send(self, writer: asyncio.StreamWriter, msg: dict):
        payload = json.dumps(msg).encode("utf-8")
        writer.write(struct.pack("<I", len(payload)))
        writer.write(payload)
        await writer.drain()

    async def _shutdown(self, loop):
        print("\n[vil-sidecar] Shutting down...")
        self._draining = True
        for _ in range(50):
            if self._in_flight == 0:
                break
            await asyncio.sleep(0.1)
        loop.stop()
