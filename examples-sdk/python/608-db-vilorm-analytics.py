#!/usr/bin/env python3
"""608-db-vilorm-analytics — Python SDK equivalent
Compile: vil compile --from python --input 608-db-vilorm-analytics.py --release
"""
import os
from vil import VilPipeline, VilServer, ServiceProcess

server = VilServer("vilorm-analytics", port=8088)
analytics = server.service_process("analytics")
analytics.endpoint("POST", "/events", "log_event")
analytics.endpoint("GET", "/events/recent", "recent_events")
analytics.endpoint("GET", "/events/by-type", "events_by_type")
analytics.endpoint("GET", "/stats/daily", "daily_stats")
analytics.endpoint("GET", "/stats/unique-users", "unique_users")
analytics.endpoint("GET", "/stats/summary", "stats_summary")
server.compile()
