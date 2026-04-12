#!/usr/bin/env lua
-- Hospital Appointment Scheduler — Lua Sidecar (stdin/stdout line-delimited JSON, pooled)
-- UDS+SHM mode requires luasocket (vil_sidecar_sdk.lua), falls back to stdin/stdout

local function extract(json_str, key)
    local pattern = '"' .. key .. '"%s*:%s*"([^"]*)"'
    return json_str:match(pattern) or ""
end

local function schedule(line)
    local patient = extract(line, "patient_name")
    if patient == "" then patient = "Unknown" end
    local department = extract(line, "department")
    if department == "" then department = "General" end
    local slot = os.date("%Y-%m-%d %H:00", os.time() + 86400)
    local appointment_id = string.format("APT-%04X", os.time() % 0xFFFF)
    return string.format(
        '{"appointment_id":"%s","patient":"%s","department":"%s","scheduled_at":"%s","status":"confirmed"}',
        appointment_id, patient, department, slot
    )
end

-- Stdin/stdout line-delimited JSON loop (pooled mode)
for line in io.lines() do
    line = line:match("^%s*(.-)%s*$")
    if #line > 0 then
        io.write(schedule(line) .. "\n")
        io.flush()
    end
end
