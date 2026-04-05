#!/usr/bin/env python3
"""Handler: PUT /tasks/:id — partial update via set_optional.

Business logic (mirrors Rust VilQuery):
  Task::q().update()
      .set_optional("title", req.title.as_deref())
      .set_optional("description", req.description.as_deref())
      .set_raw("updated_at", "datetime('now')")
      .where_eq("id", &id).execute()
"""
import json
import sqlite3
import sys
import os

task_id = os.environ.get("PATH_ID", "")
body = sys.stdin.buffer.read()
try:
    req = json.loads(body)
except:
    req = {}

if not task_id:
    task_id = req.pop("id", "")

db_path = os.environ.get("DATABASE_URL", "tasks.db").replace("sqlite:", "").split("?")[0]
conn = sqlite3.connect(db_path)
conn.row_factory = sqlite3.Row

# Build dynamic SET clause (skip None fields — mirrors set_optional)
sets = []
params = []
for field in ["title", "description"]:
    if field in req and req[field] is not None:
        sets.append(f"{field} = ?")
        params.append(req[field])

if req.get("done") is not None:
    sets.append("done = ?")
    params.append(1 if req["done"] else 0)

if sets:
    sets.append("updated_at = datetime('now')")
    sql = f"UPDATE tasks SET {', '.join(sets)} WHERE id = ?"
    params.append(task_id)
    conn.execute(sql, params)
    conn.commit()

row = conn.execute("SELECT * FROM tasks WHERE id = ?", (task_id,)).fetchone()
if row:
    print(json.dumps(dict(row)))
else:
    print(json.dumps({"error": "Task not found", "id": task_id}))
conn.close()
