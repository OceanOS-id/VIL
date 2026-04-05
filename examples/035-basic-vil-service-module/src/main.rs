// ╔════════════════════════════════════════════════════════════════════════╗
// ║  035 — Hospital Appointment System (VIL Service Module)             ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Pattern:  VX_APP                                                    ║
// ║  Token:    N/A                                                       ║
// ║  Features: vil_app! macro — generates main() with endpoint wiring    ║
// ╠════════════════════════════════════════════════════════════════════════╣
// ║  Business: A hospital needs two core services:                       ║
// ║    1. Patient Registration — new patients fill out intake forms      ║
// ║    2. Appointment Scheduling — patients book doctor visits           ║
// ║                                                                      ║
// ║  Each service is a separate module but deployed as one VIL app.      ║
// ║  The vil_app! macro generates the entire main() function with        ║
// ║  ServiceProcess + VilApp wiring — zero boilerplate.                  ║
// ║                                                                      ║
// ║  Why vil_app! matters:                                               ║
// ║    - Hospital IT teams want minimal Rust boilerplate                 ║
// ║    - Declaring endpoints should look like a config file, not code    ║
// ║    - vil_app! generates #[tokio::main] + ServiceProcess + VilApp    ║
// ║    - Developers focus on business logic, not server wiring           ║
// ╚════════════════════════════════════════════════════════════════════════╝
//
// Run:  cargo run -p vil-basic-vil-service-module
// Test: curl http://localhost:8080/
//       curl -X POST http://localhost:8080/patients/register \
//         -H 'Content-Type: application/json' \
//         -d '{"name":"John Doe","date_of_birth":"1990-05-15","insurance_id":"INS-44221"}'
//       curl -X POST http://localhost:8080/appointments/schedule \
//         -H 'Content-Type: application/json' \
//         -d '{"patient_id":1001,"doctor_id":301,"department":"cardiology","date":"2026-04-01","time_slot":"09:30"}'

use vil_server::prelude::*;

// ── Patient Domain ──────────────────────────────────────────────────────

/// Patient registration form submitted at the hospital front desk.
#[derive(Deserialize)]
struct PatientRegistration {
    name: String,
    date_of_birth: String,
    insurance_id: String,
}

/// Registered patient record returned after successful intake.
#[derive(Serialize)]
struct Patient {
    patient_id: u64,
    name: String,
    date_of_birth: String,
    insurance_id: String,
    registration_status: &'static str,
}

// ── Appointment Domain ──────────────────────────────────────────────────

/// Appointment scheduling request.
#[derive(Deserialize)]
struct ScheduleRequest {
    patient_id: u64,
    doctor_id: u64,
    department: String,
    date: String,
    time_slot: String,
}

/// Confirmed appointment returned to the patient.
#[derive(Serialize)]
struct Appointment {
    appointment_id: u64,
    patient_id: u64,
    doctor_id: u64,
    department: String,
    date: String,
    time_slot: String,
    status: &'static str,
}

// ── Service Overview ────────────────────────────────────────────────────

#[derive(Serialize)]
struct SystemOverview {
    hospital: &'static str,
    services: Vec<&'static str>,
    endpoints: Vec<&'static str>,
}

// ── Handler Implementations ─────────────────────────────────────────────

/// System overview — shows all available services and endpoints.
async fn overview() -> VilResponse<SystemOverview> {
    VilResponse::ok(SystemOverview {
        hospital: "City General Hospital — Appointment System",
        services: vec!["patient-registration", "appointment-scheduling"],
        endpoints: vec![
            "GET  /                          → this overview",
            "POST /patients/register         → register new patient",
            "POST /appointments/schedule     → book appointment",
        ],
    })
}

/// Register a new patient.
/// In production: validate insurance, check for duplicate records,
/// assign a unique medical record number (MRN).
async fn register_patient(body: ShmSlice) -> Result<VilResponse<Patient>, VilError> {
    let reg: PatientRegistration = body.json().map_err(|_| {
        VilError::bad_request("Invalid patient JSON — need name, date_of_birth, insurance_id")
    })?;

    // Generate patient ID (in production: database auto-increment or UUID)
    let patient_id = 1000 + (reg.name.len() as u64 * 7);

    Ok(VilResponse::ok(Patient {
        patient_id,
        name: reg.name,
        date_of_birth: reg.date_of_birth,
        insurance_id: reg.insurance_id,
        registration_status: "registered — ready to schedule appointments",
    }))
}

/// Schedule an appointment for a registered patient.
/// In production: check doctor availability, verify patient exists,
/// send confirmation SMS/email, update the scheduling calendar.
async fn schedule_appointment(body: ShmSlice) -> Result<VilResponse<Appointment>, VilError> {
    let req: ScheduleRequest = body.json().map_err(|_| {
        VilError::bad_request(
            "Invalid appointment JSON — need patient_id, doctor_id, department, date, time_slot",
        )
    })?;

    // Generate appointment ID
    let appointment_id = req.patient_id * 100 + req.doctor_id;

    Ok(VilResponse::ok(Appointment {
        appointment_id,
        patient_id: req.patient_id,
        doctor_id: req.doctor_id,
        department: req.department,
        date: req.date,
        time_slot: req.time_slot,
        status: "confirmed — reminder will be sent 24 hours before",
    }))
}

// ── vil_app! DSL ────────────────────────────────────────────────────────
// The vil_app! macro generates the ENTIRE main() function.
// No manual VilApp::new(), no ServiceProcess::new(), no #[tokio::main].
// Developers declare endpoints like a config file — VIL handles the rest.
vil_app! {
    name: "hospital-appointment-system",
    port: 8080,
    endpoints: {
        GET  "/"                        => overview,
        POST "/patients/register"       => register_patient,
        POST "/appointments/schedule"   => schedule_appointment,
    }
}
