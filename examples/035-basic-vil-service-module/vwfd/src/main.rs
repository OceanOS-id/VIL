// 035 — Hospital Appointment System (NativeCode for all handlers)
// Business logic matches standard src/main.rs:
//   - GET / → system overview
//   - POST /patients/register → patient_id = 1000 + (name.len * 7)
//   - POST /appointments/schedule → appointment_id = patient_id * 100 + doctor_id
use serde_json::json;

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/035-basic-vil-service-module/vwfd/workflows", 8080)
        .native("system_overview", |_| {
            Ok(json!({
                "hospital": "VIL General Hospital",
                "services": ["Patient Registration", "Appointment Scheduling"],
                "endpoints": [
                    "POST /patients/register",
                    "POST /appointments/schedule"
                ]
            }))
        })
        .native("register_patient", |input| {
            let body = &input["body"];
            let name = body["name"].as_str().unwrap_or("Patient");
            let date_of_birth = body["date_of_birth"].as_str().unwrap_or("");
            let insurance_id = body["insurance_id"].as_str().unwrap_or("");

            let patient_id = 1000 + (name.len() as u64 * 7);

            Ok(json!({
                "patient_id": patient_id,
                "name": name,
                "date_of_birth": date_of_birth,
                "insurance_id": insurance_id,
                "registration_status": "registered — ready to schedule appointments"
            }))
        })
        .native("schedule_appointment", |input| {
            let body = &input["body"];
            let patient_id = body["patient_id"].as_u64().unwrap_or(0);
            let doctor_id = body["doctor_id"].as_u64().unwrap_or(0);
            let department = body["department"].as_str().unwrap_or("");
            let date = body["date"].as_str().unwrap_or("");
            let time_slot = body["time_slot"].as_str().unwrap_or("");

            let appointment_id = patient_id * 100 + doctor_id;

            Ok(json!({
                "appointment_id": appointment_id,
                "patient_id": patient_id,
                "doctor_id": doctor_id,
                "department": department,
                "date": date,
                "time_slot": time_slot,
                "status": "confirmed — reminder will be sent 24 hours before"
            }))
        })
        .run().await;
}
