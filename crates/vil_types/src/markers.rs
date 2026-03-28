// =============================================================================
// vil_types::markers — Marker Traits and Message Contract
// =============================================================================
// Marker traits enforcing type invariants at compile-time.
// Foundation of "Safety Through Semantics, not Convention".
//
// TASK LIST:
// [x] Vasi — marker for VASI-safe types (Virtual Address Space Independent)
// [x] PodLike — marker for POD types (Plain Old Data)
// [x] LinearResource — marker for resources that must be consumed exactly once
// [x] MessageContract — trait for messages carrying static metadata
// [ ] TODO(future): ZeroCopySafe — composite marker Vasi + PodLike + !pointer
// =============================================================================

use crate::specs::MessageMeta;

/// Marker trait: this type is safe for Virtual Address Space Independent transfer.
///
/// VASI types contain no absolute pointers and can be relocated
/// without rewriting references. Must be `unsafe impl` since the
/// compiler cannot verify layout automatically.
///
/// # Safety
/// Implementors guarantee that this type:
/// - Contains no absolute pointers (`*const T`, `*mut T`, `&T`, `Box<T>`)
/// - Contains no `String`, `Vec<T>`, or standard Rust heap types
/// - All internal references are relative offsets or primitive types
pub unsafe trait Vasi {}

/// Marker trait: this type is Plain Old Data.
///
/// POD types are safe for memcpy, have no meaningful destructor,
/// and their binary representation is stable.
///
/// # Safety
/// Implementors guarantee that this type:
/// - Can be copied byte-for-byte
/// - Has no Drop implementation with significant logic
/// - Has consistent alignment
pub unsafe trait PodLike {}

/// Marker trait: linear resource that must be consumed exactly once.
///
/// Linear resources cannot be cloned and must not be silently ignored.
/// Used with `#[must_use]` to ensure ownership lifecycle is not
/// broken silently.
pub trait LinearResource {}

/// Trait for messages carrying a static metadata contract.
///
/// Every message registered in VIL must implement this trait
/// to provide layout, name, and transfer capability information.
///
/// # Example
/// ```ignore
/// impl MessageContract for CameraFrame {
///     const META: MessageMeta = MessageMeta {
///         name: "CameraFrame",
///         layout: LayoutProfile::Relative,
///         transfer_caps: &[TransferMode::LoanWrite, TransferMode::LoanRead],
///     };
/// }
/// ```
pub trait MessageContract {
    const META: MessageMeta;
}

// --- Blanket unsafe impls for primitives ---

// SAFETY: Primitive types (u8..u128, i8..i128, f32, f64, bool) contain no pointers
// and have stable representations across address spaces.
unsafe impl Vasi for u8 {}
unsafe impl Vasi for u16 {}
unsafe impl Vasi for u32 {}
unsafe impl Vasi for u64 {}
unsafe impl Vasi for u128 {}
unsafe impl Vasi for i8 {}
unsafe impl Vasi for i16 {}
unsafe impl Vasi for i32 {}
unsafe impl Vasi for i64 {}
unsafe impl Vasi for i128 {}
unsafe impl Vasi for f32 {}
unsafe impl Vasi for f64 {}
unsafe impl Vasi for bool {}

// SAFETY: Primitive types are valid for any bit pattern and can be safely zeroed/copied.
unsafe impl PodLike for u8 {}
unsafe impl PodLike for u16 {}
unsafe impl PodLike for u32 {}
unsafe impl PodLike for u64 {}
unsafe impl PodLike for u128 {}
unsafe impl PodLike for i8 {}
unsafe impl PodLike for i16 {}
unsafe impl PodLike for i32 {}
unsafe impl PodLike for i64 {}
unsafe impl PodLike for i128 {}
unsafe impl PodLike for f32 {}
unsafe impl PodLike for f64 {}
unsafe impl PodLike for bool {}
