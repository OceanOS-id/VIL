// =============================================================================
// vil_types::wrappers — Wrapper Types for Zero-Copy Lifecycle
// =============================================================================
// Wrapper types encapsulating the VIL zero-copy lifecycle:
//   loan_uninit → write → publish → recv → release/recycle
//
// These wrappers enforce ownership discipline via API: Loaned cannot be
// cloned, Published marks that data has been published (ownership transferred).
//
// TASK LIST:
// [x] VSlice<T> — relative-safe slice wrapper
// [x] VRef<T> — relative-safe reference wrapper (index-based)
// [x] Loaned<T> — write-phase loan (producer writes in-place)
// [x] LoanedRead<T> — read-phase loan (consumer reads)
// [x] Published<T> — token indicating sample has been published
// [ ] TODO(future): replace Arc-backed VSlice/LoanedRead with
//     real shared-memory offset-based implementation
// =============================================================================

use core::fmt;
use core::marker::PhantomData;
use std::sync::Arc;

use crate::enums::*;
use crate::ids::*;
use crate::markers::*;
use crate::specs::MessageMeta;

/// Slice wrapper safe for VIL boundaries.
///
/// Uses `bytes::Bytes` as the backing store, enabling zero-copy (O(1))
/// conversion to/from network buffers.
#[derive(Clone, PartialEq, Eq)]
pub struct VSlice<T> {
    data: bytes::Bytes,
    _marker: PhantomData<T>,
}

impl<T: 'static> VSlice<T> {
    /// Create a VSlice from a Vec.
    pub fn from_vec(value: Vec<T>) -> Self {
        let size = std::mem::size_of::<T>();
        let len = value.len() * size;

        // Fast path for u8
        if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u8>() {
            // SAFETY: Vec<T> and Vec<u8> have identical layout. T is constrained to types where all bit patterns are valid (PodLike).
            let bytes =
                bytes::Bytes::from(unsafe { std::mem::transmute::<Vec<T>, Vec<u8>>(value) });
            return Self {
                data: bytes,
                _marker: PhantomData,
            };
        }

        // Fallback: Copy for non-u8 types (rare on hot-path network I/O)
        let slice = unsafe { std::slice::from_raw_parts(value.as_ptr() as *const u8, len) };
        let bytes = bytes::Bytes::copy_from_slice(slice);
        Self {
            data: bytes,
            _marker: PhantomData,
        }
    }

    /// Create a VSlice from bytes::Bytes (true zero-copy).
    pub fn from_bytes(bytes: bytes::Bytes) -> Self {
        Self {
            data: bytes,
            _marker: PhantomData,
        }
    }

    /// Access data as a regular slice.
    pub fn as_slice(&self) -> &[T] {
        let slice = self.data.as_ref();
        let size = std::mem::size_of::<T>();
        // SAFETY: We assume the Bytes buffer alignment is sufficient for T.
        // For T=u8, this is always safe.
        unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const T, slice.len() / size) }
    }

    /// Take a sub-slice (zero-copy).
    pub fn slice_bytes(&self, range: std::ops::Range<usize>) -> Self {
        Self {
            data: self.data.slice(range),
            _marker: PhantomData,
        }
    }

    /// Convert back to bytes::Bytes (true zero-copy).
    pub fn to_bytes(&self) -> bytes::Bytes {
        self.data.clone()
    }

    /// Number of elements.
    pub fn len(&self) -> usize {
        let size = std::mem::size_of::<T>();
        self.data.len() / size
    }

    /// Whether the slice is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T> fmt::Debug for VSlice<T>
where
    T: fmt::Debug + 'static,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice().iter()).finish()
    }
}

// SAFETY: VSlice<T> is backed by bytes::Bytes with no absolute pointers. T: Vasi/PodLike ensures inner type is safe.
unsafe impl<T: Vasi> Vasi for VSlice<T> {}
unsafe impl<T: PodLike> PodLike for VSlice<T> {}

/// Index-based reference wrapper safe for VIL boundaries.
///
/// VRef stores an index (logical offset), not an absolute pointer.
/// Used for internal references in `relative` layouts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VRef<T> {
    index: usize,
    _marker: PhantomData<T>,
}

impl<T> VRef<T> {
    pub fn new(index: usize) -> Self {
        Self {
            index,
            _marker: PhantomData,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

// SAFETY: VRef<T> is backed by bytes::Bytes. Same reasoning as VSlice.
unsafe impl<T: Vasi> Vasi for VRef<T> {}
unsafe impl<T: PodLike> PodLike for VRef<T> {}

/// Write-phase loan: producer borrows a slot to write data in-place.
///
/// Lifecycle:
/// 1. `loan_uninit()` -> Loaned<T> (uninitialized)
/// 2. `.write(value)` -> Loaned<T> (initialized)
/// 3. `publish()` -> Published<T> (ownership transfers to queue/heap)
///
/// **Loaned<T> MUST NOT be cloned.** This enforces single-ownership.
/// If dropped without publishing, the runtime must reclaim the slot.
#[derive(Debug)]
pub struct Loaned<T> {
    sample_id: SampleId,
    origin_port: PortId,
    value: Option<T>,
}

impl<T> Loaned<T> {
    /// Create a new uninitialized loan.
    pub fn new(sample_id: SampleId, origin_port: PortId) -> Self {
        Self {
            sample_id,
            origin_port,
            value: None,
        }
    }

    /// Write data into the loan. Returns self in initialized state.
    #[must_use = "initialized loan must be published, do not discard"]
    pub fn write(mut self, value: T) -> Self {
        self.value = Some(value);
        self
    }

    /// Whether the loan contains data.
    pub fn is_initialized(&self) -> bool {
        self.value.is_some()
    }

    pub fn sample_id(&self) -> SampleId {
        self.sample_id
    }

    pub fn origin_port(&self) -> PortId {
        self.origin_port
    }

    /// Take the value from the loan (destructive, used by runtime during publish).
    pub fn take_value(&mut self) -> Option<T> {
        self.value.take()
    }
}

// Loaned<T> intentionally does NOT impl Clone — enforces linear ownership

/// Read-phase loan: consumer reads published data.
///
/// Consumer receives `LoanedRead<T>` after `recv()`.
/// Data shared via Arc (Phase 1) — target evolution: direct reference
/// to shared exchange heap.
pub struct LoanedRead<T> {
    sample_id: SampleId,
    value: Arc<T>,
}

impl<T> LoanedRead<T> {
    pub fn new(sample_id: SampleId, value: Arc<T>) -> Self {
        Self { sample_id, value }
    }

    pub fn sample_id(&self) -> SampleId {
        self.sample_id
    }

    /// Access the loaned data (read-only).
    pub fn get(&self) -> &T {
        &self.value
    }
}

impl<T: fmt::Debug> fmt::Debug for LoanedRead<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoanedRead")
            .field("sample_id", &self.sample_id)
            .field("value", &self.value)
            .finish()
    }
}

/// Token indicating that a sample has been published to a queue.
///
/// After publish, the producer no longer owns the data — ownership
/// transfers to the runtime/consumer. Published<T> serves as proof,
/// not access.
pub struct Published<T> {
    sample_id: SampleId,
    _marker: PhantomData<T>,
}

impl<T> Published<T> {
    pub fn new(sample_id: SampleId) -> Self {
        Self {
            sample_id,
            _marker: PhantomData,
        }
    }

    pub fn sample_id(&self) -> SampleId {
        self.sample_id
    }
}

impl<T> fmt::Debug for Published<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Published")
            .field("sample_id", &self.sample_id)
            .finish()
    }
}

/// Generic Token for Stream Ingestion
#[derive(Debug, Clone)]
pub struct GenericToken {
    pub session_id: u64,
    pub is_done: bool,
    pub data: VSlice<u8>,
}

impl MessageContract for GenericToken {
    const META: MessageMeta = MessageMeta {
        name: "GenericToken",
        layout: LayoutProfile::Relative,
        transfer_caps: &[TransferMode::LoanWrite, TransferMode::LoanRead],
        is_stable: false, // Contains VSlice which has heap-allocated Bytes
        semantic_kind: SemanticKind::Message,
        memory_class: MemoryClass::PagedExchange,
    };
}

// =============================================================================
// ShmToken — Fixed-size, VASI-stable token for TRUE zero-copy SHM transport
// =============================================================================
//
// Unlike GenericToken (is_stable: false → HashMap store → copies),
// ShmToken is POD/repr(C) so it goes directly into SHM:
//   publish: memcpy token (32 bytes) into SHM region — no Arc, no HashMap
//   recv: read 32 bytes from SHM pointer — no HashMap lookup, no Arc clone
//
// Data payload is written separately to SHM; ShmToken carries the offset.
//
// Copy budget: 1 memcpy (32 bytes) per publish + 1 memcpy per recv
// vs GenericToken: Arc::new + HashMap insert + HashMap lookup + Arc clone

/// Fixed-size zero-copy token for SHM Tri-Lane transport.
///
/// 32 bytes, repr(C), is_stable=true → bypasses HashMap store entirely.
/// Data payload lives in SHM at `data_offset`; token is the descriptor.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShmToken {
    /// Session identifier (correlates request → response).
    pub session_id: u64,
    /// Offset in SHM ExchangeHeap where payload bytes start.
    pub data_offset: u64,
    /// Length of payload in bytes.
    pub data_len: u32,
    /// Status: 0=data, 1=done, 2=error.
    pub status: u8,
    /// Reserved padding for alignment.
    pub _pad: [u8; 3],
}

impl ShmToken {
    /// Size of this struct in bytes (compile-time constant).
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a data token pointing to SHM payload.
    pub fn data(session_id: u64, offset: u64, len: u32) -> Self {
        Self {
            session_id,
            data_offset: offset,
            data_len: len,
            status: 0,
            _pad: [0; 3],
        }
    }

    /// Create a done marker token.
    pub fn done(session_id: u64) -> Self {
        Self {
            session_id,
            data_offset: 0,
            data_len: 0,
            status: 1,
            _pad: [0; 3],
        }
    }

    /// Create an error marker token.
    pub fn error(session_id: u64) -> Self {
        Self {
            session_id,
            data_offset: 0,
            data_len: 0,
            status: 2,
            _pad: [0; 3],
        }
    }

    /// Is this a data token (status=0)?
    pub fn is_data(&self) -> bool {
        self.status == 0
    }

    /// Is this a done marker (status=1)?
    pub fn is_done(&self) -> bool {
        self.status == 1
    }

    /// Is this an error marker (status=2)?
    pub fn is_error(&self) -> bool {
        self.status == 2
    }
}

impl MessageContract for ShmToken {
    const META: MessageMeta = MessageMeta {
        name: "ShmToken",
        layout: LayoutProfile::Flat, // POD, no pointers
        transfer_caps: &[
            TransferMode::LoanWrite,
            TransferMode::LoanRead,
            TransferMode::Copy,
        ],
        is_stable: true, // → direct SHM write/read, no HashMap
        semantic_kind: SemanticKind::Message,
        memory_class: MemoryClass::PagedExchange,
    };
}
