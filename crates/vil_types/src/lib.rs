// =============================================================================
// vil_types — Foundation Types for VIL Runtime
// =============================================================================
// Provides all primitive types, marker traits, wrappers, and specifications
// that form the inter-crate contract within the VIL workspace.
//
// Architecture: These types are the shared vocabulary used by
// vil_shm, vil_queue, vil_registry, vil_rt, and the full stack.
//
// Modules:
//   ids.rs      — Identity types (ProcessId, PortId, SampleId, etc.)
//   enums.rs    — Domain enums (LayoutProfile, TransferMode, QueueKind, etc.)
//   specs.rs    — Composite specs (PortSpec, ProcessSpec, Descriptor, etc.)
//   markers.rs  — Marker traits (Vasi, PodLike, LinearResource, MessageContract)
//   wrappers.rs — Wrapper types (VSlice, VRef, Loaned, LoanedRead, Published)
//
// TASK LIST:
// [x] Identity types
// [x] Domain enums
// [x] Composite specs
// [x] Marker traits & MessageContract
// [x] Wrapper types (Loaned, LoanedRead, Published, VSlice, VRef)
// [ ] TODO(future): RelativeOffset<T> for real shared-memory addressing
// [x] VStr for relative-safe string
// [ ] TODO(future): VArray<T, N> for fixed-size relative array
// [ ] TODO(future): VOption<T> for relative-safe optional
// =============================================================================

pub mod enums;
pub mod faults;
pub mod ids;
pub mod markers;
pub mod signals;
pub mod specs;
pub mod vstr;
pub mod wrappers;

// Re-export all public items for ergonomic use
pub use enums::*;
pub use faults::*;
pub use ids::*;
pub use markers::*;
pub use signals::*;
pub use specs::*;
pub use vstr::VStr;
pub use wrappers::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation_and_ordering() {
        let a = ProcessId(1);
        let b = ProcessId(2);
        assert!(a < b);
        assert_eq!(a, ProcessId(1));

        let p1 = PortId(10);
        let p2 = PortId(20);
        assert!(p1 < p2);
    }

    #[test]
    fn test_loaned_write_and_take() {
        let loan = Loaned::<u64>::new(SampleId(1), PortId(1));
        assert!(!loan.is_initialized());

        let loan = loan.write(42u64);
        assert!(loan.is_initialized());

        let mut loan = loan;
        let val = loan.take_value();
        assert_eq!(val, Some(42));
        assert!(!loan.is_initialized());
    }

    #[test]
    fn test_published_creation() {
        let pub_token = Published::<u32>::new(SampleId(99));
        assert_eq!(pub_token.sample_id(), SampleId(99));
    }

    #[test]
    fn test_vslice_from_vec() {
        let vs = VSlice::from_vec(vec![1u8, 2, 3, 4]);
        assert_eq!(vs.len(), 4);
        assert_eq!(vs.as_slice(), &[1, 2, 3, 4]);
        assert!(!vs.is_empty());
    }

    #[test]
    fn test_vref_index() {
        let r = VRef::<u32>::new(42);
        assert_eq!(r.index(), 42);
    }

    #[test]
    fn test_descriptor_fields() {
        let d = Descriptor {
            sample_id: SampleId(1),
            origin_host: HostId(0),
            origin_port: PortId(2),
            lineage_id: 100,
            publish_ts: 0,
        };
        assert_eq!(d.sample_id, SampleId(1));
        assert_eq!(d.lineage_id, 100);
    }

    #[test]
    fn test_observability_spec_default() {
        let obs = ObservabilitySpec::default();
        assert!(obs.tracing);
        assert!(obs.metrics);
        assert!(obs.lineage);
        assert!(!obs.audit_sample_handoff);
        assert_eq!(obs.latency_class, LatencyClass::Normal);
    }

    #[test]
    fn test_message_meta() {
        let meta = MessageMeta {
            name: "TestMsg",
            layout: LayoutProfile::Flat,
            transfer_caps: &[TransferMode::LoanWrite, TransferMode::Copy],
            is_stable: true,
            semantic_kind: SemanticKind::Message,
            memory_class: MemoryClass::PagedExchange,
        };
        assert_eq!(meta.name, "TestMsg");
        assert_eq!(meta.layout, LayoutProfile::Flat);
        assert_eq!(meta.transfer_caps.len(), 2);
    }
}
