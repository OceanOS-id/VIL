"""
VilSidecar — Main sidecar application class.

Handles:
  1. Connect to host via UDS
  2. Send Handshake with method list
  3. Event loop: receive Invoke → dispatch → send Result
  4. Health check responses
  5. Graceful drain and shutdown
"""

import json
import signal
import time
import traceback
from typing import Callable, Dict, Any, Optional

from vil_sidecar.protocol import (
    SidecarConnection,
    handshake_message,
    health_ok_message,
    result_ok_message,
    result_error_message,
    drained_message,
    ProtocolError,
)
from vil_sidecar.shm import ShmRegion


class VilSidecar:
    """
    VIL Sidecar application.

    Usage:
        app = VilSidecar("fraud-checker")

        @app.handler("fraud_check")
        def fraud_check(request: dict) -> dict:
            return {"score": 0.95}

        app.run()
    """

    def __init__(
        self,
        name: str,
        version: str = "1.0.0",
        socket_path: Optional[str] = None,
        auth_token: Optional[str] = None,
    ):
        self.name = name
        self.version = version
        self.socket_path = socket_path or f"/tmp/vil_sidecar_{name}.sock"
        self.auth_token = auth_token

        self._handlers: Dict[str, Callable] = {}
        self._conn: Optional[SidecarConnection] = None
        self._shm: Optional[ShmRegion] = None
        self._running = True
        self._draining = False

        # Metrics
        self._total_processed = 0
        self._total_errors = 0
        self._in_flight = 0
        self._start_time = time.time()

    def handler(self, method_name: str):
        """
        Register a handler function for a method name.

        Usage:
            @app.handler("fraud_check")
            def fraud_check(request: dict) -> dict:
                return {"score": 0.95}
        """
        def decorator(func):
            self._handlers[method_name] = func
            return func
        return decorator

    def add_handler(self, method_name: str, func: Callable):
        """Register a handler function programmatically."""
        self._handlers[method_name] = func

    def run(self):
        """
        Connect to host, handshake, and start the event loop.

        This blocks until the host sends Shutdown or the process is interrupted.
        """
        # Handle SIGTERM/SIGINT gracefully
        signal.signal(signal.SIGTERM, self._signal_handler)
        signal.signal(signal.SIGINT, self._signal_handler)

        print(f"[vil-sidecar] {self.name} v{self.version}")
        print(f"[vil-sidecar] methods: {list(self._handlers.keys())}")
        print(f"[vil-sidecar] connecting to {self.socket_path}")

        try:
            # Connect to host
            self._conn = SidecarConnection.connect(self.socket_path)
            print(f"[vil-sidecar] connected")

            # Send handshake
            methods = list(self._handlers.keys())
            capabilities = ["async"] if len(self._handlers) > 0 else []
            self._conn.send(
                handshake_message(
                    name=self.name,
                    version=self.version,
                    methods=methods,
                    capabilities=capabilities,
                    auth_token=self.auth_token,
                )
            )

            # Wait for HandshakeAck
            ack = self._conn.recv()
            if ack.get("type") != "HandshakeAck" or not ack.get("accepted"):
                reason = ack.get("reject_reason", "unknown")
                raise ProtocolError(f"handshake rejected: {reason}")

            # Open SHM region
            shm_path = ack.get("shm_path", "")
            shm_size = ack.get("shm_size", 0)
            if shm_path and shm_size > 0:
                try:
                    self._shm = ShmRegion(shm_path, shm_size)
                    print(f"[vil-sidecar] SHM region: {shm_path} ({shm_size // (1024*1024)}MB)")
                except Exception as e:
                    print(f"[vil-sidecar] WARNING: could not open SHM: {e}")
                    print(f"[vil-sidecar] falling back to JSON-over-UDS mode")

            print(f"[vil-sidecar] ready — serving {len(methods)} methods")

            # Event loop
            self._event_loop()

        except ProtocolError as e:
            print(f"[vil-sidecar] protocol error: {e}")
        except ConnectionRefusedError:
            print(f"[vil-sidecar] connection refused: {self.socket_path}")
        except FileNotFoundError:
            print(f"[vil-sidecar] socket not found: {self.socket_path}")
        except Exception as e:
            print(f"[vil-sidecar] error: {e}")
        finally:
            self._cleanup()

    def _event_loop(self):
        """Main event loop: receive messages and dispatch."""
        while self._running:
            try:
                msg = self._conn.recv()
            except ProtocolError:
                if self._running:
                    print("[vil-sidecar] connection lost")
                break
            except Exception:
                if self._running:
                    print("[vil-sidecar] recv error")
                break

            msg_type = msg.get("type")

            if msg_type == "Invoke":
                self._handle_invoke(msg)
            elif msg_type == "Health":
                self._handle_health()
            elif msg_type == "Drain":
                self._handle_drain()
            elif msg_type == "Shutdown":
                print("[vil-sidecar] shutdown signal received")
                self._running = False
                break
            else:
                print(f"[vil-sidecar] unknown message type: {msg_type}")

    def _handle_invoke(self, msg: Dict[str, Any]):
        """Handle an Invoke message: dispatch to handler, send Result."""
        if self._draining:
            # Reject new work during drain
            desc = msg.get("descriptor", {})
            request_id = desc.get("request_id", 0)
            self._conn.send(
                result_error_message(request_id, "sidecar is draining")
            )
            return

        desc = msg.get("descriptor", {})
        method = msg.get("method", "")
        request_id = desc.get("request_id", 0)

        handler = self._handlers.get(method)
        if handler is None:
            self._conn.send({
                "type": "Result",
                "request_id": request_id,
                "status": "MethodNotFound",
                "descriptor": None,
                "error": f"method '{method}' not found",
            })
            return

        self._in_flight += 1

        try:
            # Read request data from SHM
            request_data = {}
            offset = desc.get("offset", 0)
            length = desc.get("len", 0)

            if self._shm and length > 0:
                try:
                    request_data = self._shm.read_json(offset, length)
                except Exception as e:
                    request_data = {"_raw_error": str(e)}
            else:
                # Fallback: no SHM, use empty request
                request_data = {}

            # Call handler
            result = handler(request_data)

            # Write response to SHM
            if self._shm and isinstance(result, dict):
                resp_offset, resp_len = self._shm.write_json(result)
                self._conn.send(
                    result_ok_message(request_id, 0, resp_offset, resp_len)
                )
            else:
                # Fallback: empty response
                self._conn.send(result_ok_message(request_id))

            self._total_processed += 1

        except Exception as e:
            self._total_errors += 1
            error_msg = f"{type(e).__name__}: {e}"
            self._conn.send(result_error_message(request_id, error_msg))
            traceback.print_exc()

        finally:
            self._in_flight -= 1

    def _handle_health(self):
        """Respond to Health check."""
        uptime = int(time.time() - self._start_time)
        self._conn.send(
            health_ok_message(
                in_flight=self._in_flight,
                total_processed=self._total_processed,
                total_errors=self._total_errors,
                uptime_secs=uptime,
            )
        )

    def _handle_drain(self):
        """Handle Drain signal: stop accepting new work."""
        print("[vil-sidecar] drain signal received, finishing in-flight requests")
        self._draining = True

        # Wait for in-flight to complete (simple busy wait)
        max_wait = 30  # seconds
        waited = 0
        while self._in_flight > 0 and waited < max_wait:
            time.sleep(0.1)
            waited += 0.1

        self._conn.send(drained_message())
        print(f"[vil-sidecar] drained (processed={self._total_processed})")

    def _signal_handler(self, signum, frame):
        """Handle SIGTERM/SIGINT."""
        print(f"\n[vil-sidecar] signal {signum} received, shutting down")
        self._running = False

    def _cleanup(self):
        """Clean up resources."""
        if self._shm:
            self._shm.close()
        if self._conn:
            self._conn.close()
        print(
            f"[vil-sidecar] {self.name} stopped "
            f"(processed={self._total_processed}, errors={self._total_errors})"
        )
