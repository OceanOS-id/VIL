"""
SHM Bridge — Shared memory access for sidecar data exchange.

Both host and sidecar mmap the same /dev/shm/vil_sc_{name} file.
Layout: [64-byte header] [data area]
Header contains atomic write cursor at offset 0.
"""

import mmap
import os
import struct
from typing import Optional, Tuple

# Header size (matches Rust ShmRegion)
HEADER_SIZE = 64


class ShmRegion:
    """Memory-mapped shared region for zero-copy data exchange with the host."""

    def __init__(self, path: str, size: int = 0):
        """
        Open an existing SHM region (created by the host).

        Args:
            path: Path to the SHM file (e.g., /dev/shm/vil_sc_fraud)
            size: Expected size (0 = auto-detect from file)
        """
        self.path = path
        fd = os.open(path, os.O_RDWR)
        try:
            stat = os.fstat(fd)
            self.size = size if size > 0 else stat.st_size
            self.mmap = mmap.mmap(fd, self.size, access=mmap.ACCESS_WRITE)
        finally:
            os.close(fd)

    def read(self, offset: int, length: int) -> bytes:
        """Read data from the SHM region at the given offset."""
        if offset + length > self.size:
            raise ValueError(
                f"read out of bounds: offset={offset}, len={length}, size={self.size}"
            )
        return self.mmap[offset : offset + length]

    def write(self, data: bytes) -> Tuple[int, int]:
        """
        Write data to the SHM region using bump allocation.
        Returns (offset, length) for the descriptor.

        Note: This uses the atomic cursor in the header for thread-safe allocation.
        For single-threaded Python sidecars, this is safe without additional locking.
        """
        length = len(data)
        aligned_len = _align_up(length, 8)

        # Read current cursor (8-byte LE uint64 at offset 0)
        cursor_bytes = self.mmap[0:8]
        cursor = struct.unpack("<Q", cursor_bytes)[0]

        # Check bounds
        if cursor + aligned_len > self.size:
            raise RuntimeError(
                f"SHM region full: need {aligned_len} bytes, "
                f"{self.size - cursor} available"
            )

        # Write data
        self.mmap[cursor : cursor + length] = data

        # Advance cursor
        new_cursor = cursor + aligned_len
        self.mmap[0:8] = struct.pack("<Q", new_cursor)

        return (cursor, length)

    def read_json(self, offset: int, length: int) -> dict:
        """Read and parse JSON data from SHM."""
        import json
        raw = self.read(offset, length)
        return json.loads(raw)

    def write_json(self, data: dict) -> Tuple[int, int]:
        """Serialize data as JSON and write to SHM."""
        import json
        raw = json.dumps(data).encode("utf-8")
        return self.write(raw)

    def close(self) -> None:
        """Close the mmap."""
        try:
            self.mmap.close()
        except Exception:
            pass

    def __enter__(self):
        return self

    def __exit__(self, *args):
        self.close()


def _align_up(value: int, align: int) -> int:
    """Align value up to the given alignment."""
    return (value + align - 1) & ~(align - 1)
