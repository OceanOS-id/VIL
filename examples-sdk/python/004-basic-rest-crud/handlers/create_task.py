#!/usr/bin/env python3
"""Handler: POST /tasks — create task via VilQuery insert.

Business logic (mirrors Rust):
  1. Parse JSON body: {title, description?}
  2. Validate title not empty
  3. Generate UUID
  4. INSERT into tasks table
  5. Fetch back created task
"""
import json
import sqlite3
import uuid
import sys
import os

body = sys.stdin.buffer.read()
req = json.loads(body)

title = req.get("title", "").strip()
if not title:
    print(json.dumps({"error": "title must not be empty"}))
    sys.exit(0)

description = req.get("description", "")
task_id = str(uuid.uuid4())

db_path = os.environ.get("DATABASE_URL", "tasks.db").replace("sqlite:", "").split("?")[0]
conn = sqlite3.connect(db_path)
conn.row_factory = sqlite3.Row

conn.execute(
    "INSERT INTO tasks (id, title, description) VALUES (?, ?, ?)",
    (task_id, title, description)
)
conn.commit()

row = conn.execute("SELECT * FROM tasks WHERE id = ?", (task_id,)).fetchone()
print(json.dumps(dict(row)))
conn.close()
