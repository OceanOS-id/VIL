// 101c-vilapp-multi-pipeline-benchmark — Swift SDK equivalent
// Compile: vil compile --from swift --input 101c-vilapp-multi-pipeline-benchmark/main.swift --release

let server = VilServer(name: "multi-pipeline-bench", port: 3090)
let pipeline = ServiceProcess(name: "pipeline")
server.service(pipeline)
server.compile()
