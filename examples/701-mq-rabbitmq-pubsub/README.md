# 701-mq-rabbitmq-pubsub

RabbitMQ publish + consume with `mq_log!` auto-emit.

## What it shows

- `RabbitClient::connect()` with a local AMQP URI
- `publish()` to a queue via direct (empty) exchange
- `consume()` returning a `Receiver<RabbitMessage>`
- `ack()` for message acknowledgement
- `mq_log!` auto-emitted by `vil_mq_rabbitmq` on every operation
- `StdoutDrain::resolved()` output format

## Prerequisites

RabbitMQ:

```bash
docker run -p 5672:5672 -p 15672:15672 rabbitmq:3-management
```

Management UI available at http://localhost:15672 (guest/guest).

## Run

```bash
cargo run -p example-701-mq-rabbitmq-pubsub
```

Without RabbitMQ, the example prints the config and exits gracefully.
