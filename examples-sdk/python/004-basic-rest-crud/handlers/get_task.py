#!/usr/bin/env python3
"""Handler: GET /tasks/:id — get task by primary key.

Business logic (mirrors Rust):
  Task::find_by_id(pool, &id)
"""
import json
import sqlite3
import sys
import os

# Path parameter comes via env or stdin protocol
task_id = os.environ.get("PATH_ID", "")
if not task_id:
    body = sys.stdin.buffer.read()
    try:
        req = json.loads(body)
        task_id = req.get("id", "")
    except:
        pass

db_path = os.environ.get("DATABASE_URL", "tasks.db").replace("sqlite:", "").split("?")[0]
conn = sqlite3.connect(db_path)
conn.row_factory = sqlite3.Row

row = conn.execute("SELECT * FROM tasks WHERE id = ?", (task_id,)).fetchone()
if row:
    print(json.dumps(dict(row)))
else:
    print(json.dumps({"error": "Task not found", "id": task_id}))
conn.close()
