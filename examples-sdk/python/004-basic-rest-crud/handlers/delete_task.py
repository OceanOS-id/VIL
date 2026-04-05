#!/usr/bin/env python3
"""Handler: DELETE /tasks/:id — delete by primary key.

Business logic (mirrors Rust):
  Task::delete(pool, &id)
"""
import json
import sqlite3
import sys
import os

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

cursor = conn.execute("DELETE FROM tasks WHERE id = ?", (task_id,))
conn.commit()

if cursor.rowcount > 0:
    print(json.dumps({"deleted": True, "id": task_id}))
else:
    print(json.dumps({"error": "Task not found", "id": task_id}))
conn.close()
