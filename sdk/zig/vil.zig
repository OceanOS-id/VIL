// =============================================================================
// VIL SDK for Zig — Single-file, zero-dependency SDK.
//
// Usage:
//   zig run main.zig                                  — compiles via `vil compile`
//   VIL_COMPILE_MODE=manifest zig run main.zig        — prints YAML manifest
//
// API:
//   var p = VilPipeline.init("name", 3080);
//   p.sink("webhook", 3080, "/trigger");
//   p.source("inference", "http://...", .{ .format = "sse", .json_tap = "..." });
//   p.route("webhook.trigger_out", "inference.trigger_in", "LoanWrite");
//   p.compile();
// =============================================================================

const std = @import("std");

const MAX_NODES = 16;
const MAX_ROUTES = 32;

pub const SourceOpts = struct {
    format: ?[]const u8 = "sse",
    json_tap: ?[]const u8 = null,
    dialect: ?[]const u8 = null,
};

pub const Node = struct {
    name: []const u8,
    type_: []const u8,
    port: u16,
    path: ?[]const u8,
    url: ?[]const u8,
    format: ?[]const u8,
    json_tap: ?[]const u8,
    dialect: ?[]const u8,
};

pub const Route = struct {
    from: []const u8,
    to: []const u8,
    mode: []const u8,
};

pub const VilPipeline = struct {
    name: []const u8,
    port: u16,
    nodes: [MAX_NODES]Node = undefined,
    node_count: usize = 0,
    routes: [MAX_ROUTES]Route = undefined,
    route_count: usize = 0,

    pub fn init(name: []const u8, port: u16) VilPipeline {
        return VilPipeline{ .name = name, .port = port };
    }

    pub fn sink(self: *VilPipeline, node_name: []const u8, port: u16, path: []const u8) void {
        if (self.node_count < MAX_NODES) {
            self.nodes[self.node_count] = Node{
                .name = node_name, .type_ = "http_sink", .port = port, .path = path,
                .url = null, .format = null, .json_tap = null, .dialect = null,
            };
            self.node_count += 1;
            self.port = port;
        }
    }

    pub fn source(self: *VilPipeline, node_name: []const u8, url: []const u8, opts: SourceOpts) void {
        if (self.node_count < MAX_NODES) {
            self.nodes[self.node_count] = Node{
                .name = node_name, .type_ = "http_source", .port = 0, .path = null,
                .url = url, .format = opts.format, .json_tap = opts.json_tap, .dialect = opts.dialect,
            };
            self.node_count += 1;
        }
    }

    pub fn route(self: *VilPipeline, from: []const u8, to: []const u8, mode: []const u8) void {
        if (self.route_count < MAX_ROUTES) {
            self.routes[self.route_count] = Route{ .from = from, .to = to, .mode = mode };
            self.route_count += 1;
        }
    }

    pub fn compile(self: *VilPipeline) void {
        const stdout = std.io.getStdOut().writer();
        const env = std.posix.getenv("VIL_COMPILE_MODE");
        const is_manifest = if (env) |v| std.mem.eql(u8, v, "manifest") else false;

        self.writeYaml(stdout) catch {};
        if (!is_manifest) {
            stdout.print("  Compiling: {s}\n", .{self.name}) catch {};
        }
    }

    pub fn writeYaml(self: *VilPipeline, writer: anytype) !void {
        try writer.print("vil_version: \"6.0.0\"\nname: {s}\nport: {d}\ntoken: shm\n", .{ self.name, self.port });
        if (self.node_count > 0) {
            try writer.writeAll("\nnodes:\n");
            for (0..self.node_count) |i| {
                const nd = self.nodes[i];
                try writer.print("  {s}:\n    type: {s}\n", .{ nd.name, nd.type_ });
                if (nd.port != 0) try writer.print("    port: {d}\n", .{nd.port});
                if (nd.path) |p| try writer.print("    path: \"{s}\"\n", .{p});
                if (nd.url) |u| try writer.print("    url: \"{s}\"\n", .{u});
                if (nd.format) |f| try writer.print("    format: {s}\n", .{f});
                if (nd.json_tap) |j| try writer.print("    json_tap: \"{s}\"\n", .{j});
                if (nd.dialect) |d| try writer.print("    dialect: {s}\n", .{d});
            }
        }
        if (self.route_count > 0) {
            try writer.writeAll("\nroutes:\n");
            for (0..self.route_count) |i| {
                const r = self.routes[i];
                try writer.print("  - from: {s}\n    to: {s}\n    mode: {s}\n", .{ r.from, r.to, r.mode });
            }
        }
    }
};

pub const ServiceEndpoint = struct {
    method: []const u8,
    path: []const u8,
    handler: []const u8,
};

pub const ServiceProcess = struct {
    name: []const u8,
    prefix: []const u8,
    endpoints: [MAX_NODES]ServiceEndpoint = undefined,
    endpoint_count: usize = 0,

    pub fn init(name: []const u8, prefix: []const u8) ServiceProcess {
        return ServiceProcess{ .name = name, .prefix = prefix };
    }

    pub fn endpoint(self: *ServiceProcess, method: []const u8, path: []const u8, handler: []const u8) void {
        if (self.endpoint_count < MAX_NODES) {
            self.endpoints[self.endpoint_count] = ServiceEndpoint{ .method = method, .path = path, .handler = handler };
            self.endpoint_count += 1;
        }
    }
};

pub const VilServer = struct {
    name: []const u8,
    port: u16,

    pub fn init(name: []const u8, port: u16) VilServer {
        return VilServer{ .name = name, .port = port };
    }

    pub fn compile(self: *VilServer) void {
        const stdout = std.io.getStdOut().writer();
        const env = std.posix.getenv("VIL_COMPILE_MODE");
        const is_manifest = if (env) |v| std.mem.eql(u8, v, "manifest") else false;
        stdout.print("vil_version: \"6.0.0\"\nname: {s}\nport: {d}\n", .{ self.name, self.port }) catch {};
        if (!is_manifest) {
            stdout.print("  Compiling: {s}\n", .{self.name}) catch {};
        }
    }
};
