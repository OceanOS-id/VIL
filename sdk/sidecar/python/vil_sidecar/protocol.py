"""
VilSidecarProtocol — Wire protocol for host <-> sidecar communication.

Transport: Unix Domain Socket (UDS)
Framing: 4-byte LE length prefix + JSON payload
Data plane: /dev/shm/vil_sc_{name} (zero-copy via mmap)
"""

import json
import struct
import socket
import os
from typing import Optional, Dict, Any

# Maximum frame size (16 MB)
MAX_FRAME_SIZE = 16 * 1024 * 1024


class ProtocolError(Exception):
    """Error in sidecar protocol communication."""
    pass


class SidecarConnection:
    """Unix Domain Socket connection with length-prefixed JSON framing."""

    def __init__(self, sock: socket.socket):
        self._sock = sock

    @classmethod
    def connect(cls, socket_path: str, timeout: float = 30.0) -> "SidecarConnection":
        """Connect to the host's UDS socket."""
        sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        sock.settimeout(timeout)
        sock.connect(socket_path)
        return cls(sock)

    def send(self, message: Dict[str, Any]) -> None:
        """Send a protocol message (length-prefixed JSON)."""
        payload = json.dumps(message).encode("utf-8")
        header = struct.pack("<I", len(payload))
        self._sock.sendall(header + payload)

    def recv(self) -> Dict[str, Any]:
        """Receive a protocol message (length-prefixed JSON)."""
        header = self._recv_exact(4)
        if not header:
            raise ProtocolError("connection closed")
        length = struct.unpack("<I", header)[0]
        if length > MAX_FRAME_SIZE:
            raise ProtocolError(f"frame too large: {length} bytes")
        payload = self._recv_exact(length)
        if not payload:
            raise ProtocolError("connection closed during payload read")
        return json.loads(payload)

    def close(self) -> None:
        """Close the connection."""
        try:
            self._sock.close()
        except OSError:
            pass

    def _recv_exact(self, n: int) -> Optional[bytes]:
        """Read exactly n bytes from the socket."""
        data = b""
        while len(data) < n:
            chunk = self._sock.recv(n - len(data))
            if not chunk:
                return None
            data += chunk
        return data

    def fileno(self) -> int:
        return self._sock.fileno()


def handshake_message(
    name: str,
    version: str,
    methods: list,
    capabilities: list = None,
    auth_token: str = None,
) -> Dict[str, Any]:
    """Build a Handshake message."""
    msg = {
        "type": "Handshake",
        "name": name,
        "version": version,
        "methods": methods,
        "capabilities": capabilities or [],
    }
    if auth_token:
        msg["auth_token"] = auth_token
    else:
        msg["auth_token"] = None
    return msg


def health_ok_message(
    in_flight: int = 0,
    total_processed: int = 0,
    total_errors: int = 0,
    uptime_secs: int = 0,
) -> Dict[str, Any]:
    """Build a HealthOk response message."""
    return {
        "type": "HealthOk",
        "in_flight": in_flight,
        "total_processed": total_processed,
        "total_errors": total_errors,
        "uptime_secs": uptime_secs,
    }


def result_ok_message(
    request_id: int,
    region_id: int = 0,
    offset: int = 0,
    length: int = 0,
) -> Dict[str, Any]:
    """Build a Result (Ok) response message."""
    return {
        "type": "Result",
        "request_id": request_id,
        "status": "Ok",
        "descriptor": {
            "request_id": request_id,
            "region_id": region_id,
            "_pad0": 0,
            "offset": offset,
            "len": length,
            "method_hash": 0,
            "timeout_ms": 0,
            "flags": 0,
        },
        "error": None,
    }


def result_error_message(request_id: int, error: str) -> Dict[str, Any]:
    """Build a Result (Error) response message."""
    return {
        "type": "Result",
        "request_id": request_id,
        "status": "Error",
        "descriptor": None,
        "error": error,
    }


def drained_message() -> Dict[str, Any]:
    """Build a Drained response message."""
    return {"type": "Drained"}
