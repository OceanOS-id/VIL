// 009 — OJK SLIK v2.1 Regulatory Mapping (NDJSON stream + 11-field mapping)
// Business logic matches standard src/main.rs:
//   Field mapping (Kamus Data SLIK):
//     id → no_rekening, nik → nik_debitur, nama_lengkap → nama_debitur,
//     jenis_fasilitas → jenis_fasilitas, jumlah_kredit → plafon,
//     mata_uang → mata_uang (default "IDR"), saldo_outstanding → baki_debet,
//     kolektabilitas → kualitas_kredit, kode_cabang → kode_kantor_cabang,
//     tanggal_mulai → tanggal_mulai, tanggal_jatuh_tempo → tanggal_jatuh_tempo
//   Validation: NIK 16-digit, saldo >= 0, kolektabilitas 1-5
//   Output: _slik_version "v2.1", _validated boolean
use serde_json::{json, Value};

fn map_to_slik_v21(input: &Value) -> Result<Value, String> {
    let records = input.get("records").and_then(|v| v.as_array());

    let mapped: Vec<Value> = match records {
        Some(arr) => arr.iter().map(|rec| {
            // 11-field OJK SLIK v2.1 mapping
            let nik = rec["nik"].as_str().unwrap_or("");
            let saldo = rec["saldo_outstanding"].as_f64().unwrap_or(-1.0);
            let kol = rec["kolektabilitas"].as_u64().unwrap_or(0);

            // Validation: NIK 16 digits, saldo >= 0, kolektabilitas 1-5
            let nik_valid = nik.len() == 16 && nik.chars().all(|c| c.is_ascii_digit());
            let saldo_valid = saldo >= 0.0;
            let kol_valid = kol >= 1 && kol <= 5;
            let validated = nik_valid && saldo_valid && kol_valid;

            let mata_uang = rec["mata_uang"].as_str().unwrap_or("IDR");

            json!({
                "no_rekening": rec["id"],
                "nik_debitur": rec["nik"],
                "nama_debitur": rec["nama_lengkap"],
                "jenis_fasilitas": rec["jenis_fasilitas"],
                "plafon": rec["jumlah_kredit"],
                "mata_uang": mata_uang,
                "baki_debet": rec["saldo_outstanding"],
                "kualitas_kredit": rec["kolektabilitas"],
                "kode_kantor_cabang": rec["kode_cabang"],
                "tanggal_mulai": rec["tanggal_mulai"],
                "tanggal_jatuh_tempo": rec["tanggal_jatuh_tempo"],
                "_slik_version": "v2.1",
                "_validated": validated
            })
        }).collect(),
        None => vec![],
    };

    let valid_count = mapped.iter().filter(|r| r["_validated"].as_bool().unwrap_or(false)).count();

    Ok(json!({
        "total_records": mapped.len(),
        "valid_count": valid_count,
        "invalid_count": mapped.len() - valid_count,
        "slik_version": "v2.1",
        "records": mapped
    }))
}

#[tokio::main]
async fn main() {
    vil_vwfd::app("examples/009-basic-credit-regulatory-slik/vwfd/workflows", 3083)
        .native("map_to_slik_v21", map_to_slik_v21)
        .run().await;
}
