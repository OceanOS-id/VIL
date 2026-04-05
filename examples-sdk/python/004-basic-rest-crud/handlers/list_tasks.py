#!/usr/bin/env python3
"""Handler: GET /tasks — list tasks with slim projection.

Business logic (mirrors Rust VilQuery):
  Task::q().select(&["id","title","done","created_at"])
      .order_by_desc("created_at").limit(100).fetch_all()
"""
import json
import sqlite3
import os

db_path = os.environ.get("DATABASE_URL", "tasks.db").replace("sqlite:", "").split("?")[0]
conn = sqlite3.connect(db_path)
conn.row_factory = sqlite3.Row

rows = conn.execute(
    "SELECT id, title, done, created_at FROM tasks ORDER BY created_at DESC LIMIT 100"
).fetchall()

result = [dict(r) for r in rows]
print(json.dumps(result))
conn.close()
