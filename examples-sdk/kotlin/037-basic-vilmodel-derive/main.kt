// 037-basic-vilmodel-derive — Kotlin SDK equivalent
// Compile: vil compile --from kotlin --input 037-basic-vilmodel-derive/main.kt --release

fun main() {
    val server = VilServer("insurance-claim-processing", 8080)
    val claims = ServiceProcess("claims")
    claims.endpoint("POST", "/claims/submit", "submit_claim")
    claims.endpoint("GET", "/claims/sample", "sample_claim")
    server.service(claims)
    server.compile()
}
