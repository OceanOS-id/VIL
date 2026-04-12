#!/usr/bin/env lua
-- VIL Sidecar SDK — Lua
-- Connect to VIL host via UDS, exchange data via SHM, handle Invoke/Result.
--
-- Usage:
--   local sdk = dofile("vil_sidecar_sdk.lua")
--   local app = sdk.new("my-scorer")
--   app:handler("predict", function(data) return {score = 0.95} end)
--   app:run()

local socket = require("socket")
local socket_unix = require("socket.unix")

local M = {}

-- Minimal JSON encoder (no external dep)
local function json_encode(val)
    if type(val) == "table" then
        -- Check if array or object
        local is_array = (#val > 0) or next(val) == nil
        if is_array and #val > 0 then
            local parts = {}
            for _, v in ipairs(val) do parts[#parts+1] = json_encode(v) end
            return "[" .. table.concat(parts, ",") .. "]"
        else
            local parts = {}
            for k, v in pairs(val) do
                parts[#parts+1] = '"' .. tostring(k) .. '":' .. json_encode(v)
            end
            return "{" .. table.concat(parts, ",") .. "}"
        end
    elseif type(val) == "string" then
        return '"' .. val:gsub('"', '\\"') .. '"'
    elseif type(val) == "number" then
        return tostring(val)
    elseif type(val) == "boolean" then
        return val and "true" or "false"
    elseif val == nil then
        return "null"
    end
    return '"' .. tostring(val) .. '"'
end

-- Minimal JSON decoder (handles simple structures)
local function json_decode(str)
    -- Use load for simple JSON (Lua 5.3+)
    local fn = load("return " .. str:gsub('"(%w+)":', '%1='):gsub('%[', '{'):gsub('%]', '}'):gsub(':null', ':nil'):gsub(':true', ':true'):gsub(':false', ':false'))
    if fn then return fn() end
    return str
end

function M.new(name, version)
    local self = {
        name = name,
        version = version or "1.0.0",
        handlers = {},
        conn = nil,
    }
    setmetatable(self, {__index = M})
    return self
end

function M:handler(method, fn)
    self.handlers[method] = fn
end

function M:run()
    local sock_path = os.getenv("VIL_SIDECAR_SOCKET") or
        ("/tmp/vil_sidecar_" .. self.name .. ".sock")

    self.conn = socket_unix()
    assert(self.conn:connect(sock_path))

    -- Send Handshake
    local methods = {}
    for k, _ in pairs(self.handlers) do methods[#methods+1] = k end
    self:_send({type="Handshake", name=self.name, version=self.version,
                methods=methods, capabilities={}, auth_token=nil})

    -- Receive HandshakeAck
    local ack = self:_recv()
    -- Note: SHM setup omitted for Lua (use inline data in messages)

    -- Main loop
    while true do
        local ok, msg = pcall(function() return self:_recv() end)
        if not ok then break end
        if msg.type == "Invoke" then
            self:_handle_invoke(msg)
        elseif msg.type == "Health" then
            self:_send({type="HealthOk", in_flight=0, total_processed=0, total_errors=0, uptime_secs=0})
        elseif msg.type == "Shutdown" then
            break
        end
    end
end

function M:_handle_invoke(msg)
    local method = msg.method
    local request_id = msg.descriptor.request_id
    local handler = self.handlers[method]
    if not handler then
        self:_send({type="Result", request_id=request_id, status="MethodNotFound",
                    descriptor=nil, error="no handler for '" .. method .. "'"})
        return
    end
    local ok, result = pcall(handler, {})  -- TODO: read from SHM
    if ok then
        self:_send({type="Result", request_id=request_id, status="Ok",
                    descriptor={request_id=request_id, region_id=0, _pad0=0,
                                offset=0, len=0, method_hash=0, timeout_ms=0, flags=0},
                    error=nil})
    else
        self:_send({type="Result", request_id=request_id, status="Error",
                    descriptor=nil, error=tostring(result)})
    end
end

function M:_send(msg)
    local data = json_encode(msg)
    local len = #data
    local header = string.char(len % 256, math.floor(len/256) % 256,
                               math.floor(len/65536) % 256, math.floor(len/16777216) % 256)
    self.conn:send(header .. data)
end

function M:_recv()
    local header = assert(self.conn:receive(4))
    local len = header:byte(1) + header:byte(2)*256 + header:byte(3)*65536 + header:byte(4)*16777216
    local data = assert(self.conn:receive(len))
    return json_decode(data)
end

return M
