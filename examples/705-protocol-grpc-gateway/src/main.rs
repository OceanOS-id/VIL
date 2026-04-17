// ╔════════════════════════════════════════════════════════════╗
// ║  705 — Payment Processing Gateway (gRPC)                  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Domain:   FinTech — Payment Processing                    ║
// ║  Pattern:  gRPC server via tonic + vil_grpc                ║
// ║  Features: Unary RPCs, tonic codegen, GrpcGatewayBuilder  ║
// ╠════════════════════════════════════════════════════════════╣
// ║  Business: Payment gateway with Charge, GetPayment, Refund ║
// ║  Real gRPC server on port 50051.                           ║
// ╚════════════════════════════════════════════════════════════╝
//
// Run:   cargo run -p vil-protocol-grpc-gateway
// Test (grpcurl):
//   grpcurl -plaintext -d '{"customer_id":"C-001","amount_cents":5000,"currency":"USD","description":"Order #1234"}' \
//     localhost:50051 payment.PaymentService/Charge
//   grpcurl -plaintext -d '{"payment_id":"PAY-00001"}' localhost:50051 payment.PaymentService/GetPayment
//   grpcurl -plaintext -d '{"payment_id":"PAY-00001","reason":"customer request"}' \
//     localhost:50051 payment.PaymentService/Refund

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU64, Ordering};

use tonic::{Request, Response, Status};

pub mod payment {
    tonic::include_proto!("payment");
}

use payment::payment_service_server::{PaymentService, PaymentServiceServer};
use payment::{
    ChargeRequest, ChargeResponse,
    GetPaymentRequest, PaymentRecord,
    RefundRequest, RefundResponse,
};

struct PaymentState {
    payments: Mutex<HashMap<String, PaymentRecord>>,
    next_id: AtomicU64,
}

struct PaymentGateway {
    state: Arc<PaymentState>,
}

#[tonic::async_trait]
impl PaymentService for PaymentGateway {
    async fn charge(
        &self,
        request: Request<ChargeRequest>,
    ) -> Result<Response<ChargeResponse>, Status> {
        let req = request.into_inner();

        if req.amount_cents <= 0 {
            return Err(Status::invalid_argument("amount_cents must be positive"));
        }
        if req.customer_id.is_empty() {
            return Err(Status::invalid_argument("customer_id required"));
        }

        let id_num = self.state.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let payment_id = format!("PAY-{:05}", id_num);

        let status = if req.amount_cents > 1_000_000 { "declined" } else { "approved" };

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let record = PaymentRecord {
            payment_id: payment_id.clone(),
            customer_id: req.customer_id.clone(),
            amount_cents: req.amount_cents,
            currency: req.currency.clone(),
            status: status.into(),
            description: req.description,
            created_at: now,
        };

        self.state
            .payments
            .lock()
            .unwrap()
            .insert(payment_id.clone(), record);

        Ok(Response::new(ChargeResponse {
            payment_id,
            status: status.into(),
            amount_cents: req.amount_cents,
            currency: req.currency,
            customer_id: req.customer_id,
            created_at: now,
        }))
    }

    async fn get_payment(
        &self,
        request: Request<GetPaymentRequest>,
    ) -> Result<Response<PaymentRecord>, Status> {
        let req = request.into_inner();
        let payments = self.state.payments.lock().unwrap();
        let record = payments
            .get(&req.payment_id)
            .ok_or_else(|| Status::not_found(format!("payment {} not found", req.payment_id)))?;
        Ok(Response::new(record.clone()))
    }

    async fn refund(
        &self,
        request: Request<RefundRequest>,
    ) -> Result<Response<RefundResponse>, Status> {
        let req = request.into_inner();
        let mut payments = self.state.payments.lock().unwrap();
        let record = payments
            .get_mut(&req.payment_id)
            .ok_or_else(|| Status::not_found(format!("payment {} not found", req.payment_id)))?;

        if record.status == "refunded" {
            return Err(Status::already_exists("already refunded"));
        }
        if record.status != "approved" {
            return Err(Status::failed_precondition(format!(
                "cannot refund {} payment",
                record.status
            )));
        }

        record.status = "refunded".into();
        let refund_id = format!("REF-{}", &req.payment_id[4..]);

        Ok(Response::new(RefundResponse {
            refund_id,
            payment_id: req.payment_id,
            status: "refunded".into(),
            reason: req.reason,
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let state = Arc::new(PaymentState {
        payments: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(0),
    });

    let service = PaymentGateway { state };

    let gateway = vil_grpc::GrpcGatewayBuilder::new()
        .listen(50051)
        .health_check(true);

    let addr = gateway.addr();
    tracing::info!("gRPC PaymentService listening on {}", addr);

    gateway
        .build()
        .add_service(PaymentServiceServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
