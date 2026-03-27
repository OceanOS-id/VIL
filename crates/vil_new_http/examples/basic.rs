use vil_ir::builder::{MessageBuilder, WorkflowBuilder};
use vil_new_http::{HttpSinkBuilder, WorkflowBuilderExt};
use vil_types::LayoutProfile;

fn main() {
    let sink_builder = HttpSinkBuilder::new("WebhookTrigger")
        .port(8080)
        .path("/trigger")
        .out_port("webhook_out")
        .queue_capacity(8192);

    let workflow = WorkflowBuilder::new("NewHttpBenchmark")
        .add_message(
            MessageBuilder::new("GenericToken")
                .layout(LayoutProfile::Relative)
                .build(),
        )
        .add_http_sink(sink_builder)
        .build();

    println!("vil_new_http Workflow built successfully with {} interfaces.", workflow.interfaces.len());
}
