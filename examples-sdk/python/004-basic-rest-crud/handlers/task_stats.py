#!/usr/bin/env python3
"""Handler: GET /tasks/stats — aggregate statistics.

Business logic (mirrors Rust VilQuery scalar):
  Task::count(pool)
  Task::q().select_expr("COUNT(*)").where_eq_val("done", 1).scalar()
"""
import json
import sqlite3
import os

db_path = os.environ.get("DATABASE_URL", "tasks.db").replace("sqlite:", "").split("?")[0]
conn = sqlite3.connect(db_path)

total = conn.execute("SELECT COUNT(*) FROM tasks").fetchone()[0]
done = conn.execute("SELECT COUNT(*) FROM tasks WHERE done = 1").fetchone()[0]

print(json.dumps({
    "total": total,
    "done": done,
    "pending": total - done,
}))
conn.close()
