# 702-mq-sqs-send-receive

AWS SQS send / receive compatible with LocalStack.

## What it shows

- `SqsClient::from_config()` with LocalStack endpoint
- `send_message`, `receive_messages`, `delete_message`
- `mq_log!` auto-emitted by `vil_mq_sqs` on every operation
- `StdoutDrain::resolved()` output format

## Prerequisites

LocalStack:

```bash
docker run -p 4566:4566 localstack/localstack

# Create the queue
aws --endpoint-url=http://localhost:4566 \
    sqs create-queue --queue-name vil-tasks
```

Alternatively configure real AWS credentials and set the correct queue URL.

## Run

```bash
cargo run -p example-702-mq-sqs-send-receive
```

Without LocalStack, the example prints the config and exits gracefully.
