// 037-basic-vilmodel-derive — Swift SDK equivalent
// Compile: vil compile --from swift --input 037-basic-vilmodel-derive/main.swift --release

let server = VilServer(name: "insurance-claim-processing", port: 8080)
let claims = ServiceProcess(name: "claims")
claims.endpoint(method: "POST", path: "/claims/submit", handler: "submit_claim")
claims.endpoint(method: "GET", path: "/claims/sample", handler: "sample_claim")
server.service(claims)
server.compile()
