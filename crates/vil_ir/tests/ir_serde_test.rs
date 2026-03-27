use vil_ir::WorkflowIR;
use vil_ir::builder::{WorkflowBuilder, MessageBuilder, InterfaceBuilder};
use vil_types::QueueKind;

#[test]
fn test_ir_json_roundtrip() {
    let ir = WorkflowBuilder::new("SerdeTest")
        .add_message(MessageBuilder::new("Frame")
            .add_field("id", vil_ir::core::TypeRefIR::Primitive("u64".to_string()))
            .build())
        .add_interface(InterfaceBuilder::new("VideoSource")
            .out_port("stream", "Frame")
            .queue(QueueKind::Spsc, 512)
            .done()
            .build())
        .build();

    let json = ir.to_json();
    println!("Generated JSON:\n{}", json);

    let ir_back = WorkflowIR::from_json(&json).expect("Failed to deserialize JSON");

    assert_eq!(ir.name, ir_back.name);
    assert_eq!(ir.messages.len(), ir_back.messages.len());
    assert_eq!(ir.interfaces.len(), ir_back.interfaces.len());
    
    // Verify specific content
    assert!(ir_back.messages.contains_key("Frame"));
    assert!(ir_back.interfaces.contains_key("VideoSource"));
}
