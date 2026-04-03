// =============================================================================
// VIL SDK for C# — Single-file, zero-dependency SDK for `dotnet script`.
//
// Usage:
//   dotnet script Main.cs            — compiles via `vil compile`
//   VIL_COMPILE_MODE=manifest dotnet script Main.cs  — prints YAML manifest
//
// API:
//   var p = new VilPipeline("name", 3080);
//   p.Sink("webhook", 3080, "/trigger")
//    .Source("inference", "http://...", "sse", jsonTap: "choices[0].delta.content")
//    .Route("webhook.trigger_out", "inference.trigger_in", "LoanWrite")
//    .Compile();
// =============================================================================

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.Text;

public class ServiceProcess
{
    public string Name;
    public string Prefix;
    internal List<(string Method, string Path, string Handler)> Endpoints = new();

    public ServiceProcess(string name) { Name = name; Prefix = $"/api/{name}"; }

    public ServiceProcess Endpoint(string method, string path, string handler = null)
    {
        handler ??= $"{method.ToLower()}_{path.TrimStart('/').Replace("/", "_").Replace(":", "")}";
        Endpoints.Add((method, path, handler));
        return this;
    }
}

public class VilPipeline
{
    string _name; int _port;
    List<string> _nodeOrder = new();
    Dictionary<string, (string Type, int Port, string Path, string Url, string Format, string JsonTap, string Dialect)> _nodes = new();
    List<(string From, string To, string Mode)> _routes = new();

    public VilPipeline(string name, int port) { _name = name; _port = port; }

    public VilPipeline Sink(string nodeName, int port = 3080, string path = "/trigger")
    {
        _nodes[nodeName] = ("http_sink", port, path, null, null, null, null);
        if (!_nodeOrder.Contains(nodeName)) _nodeOrder.Add(nodeName);
        _port = port;
        return this;
    }

    public VilPipeline Source(string nodeName, string url, string format = "sse",
        string jsonTap = null, string dialect = null)
    {
        _nodes[nodeName] = ("http_source", 0, null, url, format, jsonTap, dialect);
        if (!_nodeOrder.Contains(nodeName)) _nodeOrder.Add(nodeName);
        return this;
    }

    public VilPipeline Route(string from, string to, string mode)
    {
        _routes.Add((from, to, mode));
        return this;
    }

    public string ToYaml()
    {
        var sb = new StringBuilder();
        sb.AppendLine("vil_version: \"6.0.0\"");
        sb.AppendLine($"name: {_name}");
        sb.AppendLine($"port: {_port}");
        sb.AppendLine($"token: shm");
        if (_nodeOrder.Count > 0)
        {
            sb.AppendLine(); sb.AppendLine("nodes:");
            foreach (var n in _nodeOrder)
            {
                var nd = _nodes[n];
                sb.AppendLine($"  {n}:");
                sb.AppendLine($"    type: {nd.Type}");
                if (nd.Port != 0) sb.AppendLine($"    port: {nd.Port}");
                if (nd.Path != null) sb.AppendLine($"    path: \"{nd.Path}\"");
                if (nd.Url != null) sb.AppendLine($"    url: \"{nd.Url}\"");
                if (nd.Format != null) sb.AppendLine($"    format: {nd.Format}");
                if (nd.JsonTap != null) sb.AppendLine($"    json_tap: \"{nd.JsonTap}\"");
                if (nd.Dialect != null) sb.AppendLine($"    dialect: {nd.Dialect}");
            }
        }
        if (_routes.Count > 0)
        {
            sb.AppendLine(); sb.AppendLine("routes:");
            foreach (var r in _routes)
            {
                sb.AppendLine($"  - from: {r.From}");
                sb.AppendLine($"    to: {r.To}");
                sb.AppendLine($"    mode: {r.Mode}");
            }
        }
        return sb.ToString();
    }

    public void Compile()
    {
        var yaml = ToYaml();
        if (Environment.GetEnvironmentVariable("VIL_COMPILE_MODE") == "manifest")
        { Console.Write(yaml); return; }
        Console.WriteLine($"  Compiling: {_name}");
        Console.Write(yaml);
    }
}

public class VilServer
{
    string _name; int _port;
    List<ServiceProcess> _services = new();
    List<(string Name, string Kind)> _semanticTypes = new();

    public VilServer(string name, int port) { _name = name; _port = port; }

    public VilServer Service(ServiceProcess svc) { _services.Add(svc); return this; }
    public VilServer SemanticType(string name, string kind) { _semanticTypes.Add((name, kind)); return this; }

    public string ToYaml()
    {
        var sb = new StringBuilder();
        sb.AppendLine("vil_version: \"6.0.0\"");
        sb.AppendLine($"name: {_name}");
        sb.AppendLine($"port: {_port}");
        if (_semanticTypes.Count > 0)
        {
            sb.AppendLine("semantic_types:");
            foreach (var st in _semanticTypes)
            { sb.AppendLine($"  - name: {st.Name}"); sb.AppendLine($"    kind: {st.Kind}"); }
        }
        if (_services.Count > 0)
        {
            sb.AppendLine("services:");
            foreach (var svc in _services)
            {
                sb.AppendLine($"  - name: {svc.Name}");
                sb.AppendLine($"    prefix: \"{svc.Prefix}\"");
                if (svc.Endpoints.Count > 0)
                {
                    sb.AppendLine("    endpoints:");
                    foreach (var ep in svc.Endpoints)
                    {
                        sb.AppendLine($"      - method: {ep.Method}");
                        sb.AppendLine($"        path: \"{ep.Path}\"");
                        sb.AppendLine($"        handler: {ep.Handler}");
                    }
                }
            }
        }
        return sb.ToString();
    }

    public void Compile()
    {
        var yaml = ToYaml();
        if (Environment.GetEnvironmentVariable("VIL_COMPILE_MODE") == "manifest")
        { Console.Write(yaml); return; }
        Console.WriteLine($"  Compiling: {_name}");
        Console.Write(yaml);
    }
}
