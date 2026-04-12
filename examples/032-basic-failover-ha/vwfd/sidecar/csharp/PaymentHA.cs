// Payment HA Processor — C# Sidecar (stdin/stdout line-delimited JSON, pooled)
// UDS+SHM mode requires VIL C# SDK (future)
// Run: dotnet-script PaymentHA.cs
using System;
using System.Text.Json;

string line;
while ((line = Console.ReadLine()) != null) {
    line = line.Trim();
    if (string.IsNullOrEmpty(line)) continue;
    try {
        var input = JsonSerializer.Deserialize<JsonElement>(line);
        var amount = input.TryGetProperty("amount", out var a) ? a.GetDouble() : 0;
        var provider = input.TryGetProperty("provider", out var p) ? p.GetString() : "stripe";
        var paymentId = $"pay_{Guid.NewGuid().ToString("N")[..12]}";
        var result = new { payment_id = paymentId, amount, provider, status = "charged", processor = "csharp_sidecar" };
        Console.WriteLine(JsonSerializer.Serialize(result));
        Console.Out.Flush();
    } catch (Exception ex) {
        Console.WriteLine(JsonSerializer.Serialize(new { error = ex.Message }));
        Console.Out.Flush();
    }
}
