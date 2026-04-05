package handlers

import (
	"database/sql"
	"encoding/json"
)

// HandleListTasks — GET /tasks
// List tasks with slim projection (id, title, done, created_at), ordered by created_at desc.
func HandleListTasks(body []byte) interface{} {
	db := GetDB()
	if db == nil {
		return map[string]interface{}{"error": "database not connected"}
	}

	rows, err := db.Query("SELECT id, title, done, created_at FROM tasks ORDER BY created_at DESC LIMIT 100")
	if err != nil {
		return map[string]interface{}{"error": err.Error()}
	}
	defer rows.Close()

	var tasks []map[string]interface{}
	for rows.Next() {
		var id, title, createdAt string
		var done int64
		if err := rows.Scan(&id, &title, &done, &createdAt); err != nil {
			continue
		}
		tasks = append(tasks, map[string]interface{}{
			"id":         id,
			"title":      title,
			"done":       done,
			"created_at": createdAt,
		})
	}
	if tasks == nil {
		tasks = []map[string]interface{}{}
	}
	return tasks
}

// HandleCreateTask — POST /tasks
// Create a new task from JSON body {title, description?}.
func HandleCreateTask(body []byte) interface{} {
	db := GetDB()
	if db == nil {
		return map[string]interface{}{"error": "database not connected"}
	}

	var req struct {
		Title       string `json:"title"`
		Description string `json:"description"`
	}
	json.Unmarshal(body, &req)

	if req.Title == "" {
		return map[string]interface{}{"error": "title must not be empty"}
	}

	id := GenerateUUID()
	_, err := db.Exec(
		"INSERT INTO tasks (id, title, description) VALUES (?, ?, ?)",
		id, req.Title, req.Description,
	)
	if err != nil {
		return map[string]interface{}{"error": err.Error()}
	}

	return fetchTask(db, id)
}

// HandleGetTask — GET /tasks/:id
// Get a single task by primary key.
func HandleGetTask(body []byte) interface{} {
	db := GetDB()
	if db == nil {
		return map[string]interface{}{"error": "database not connected"}
	}

	var req struct {
		ID string `json:"id"`
	}
	json.Unmarshal(body, &req)

	task := fetchTask(db, req.ID)
	if task == nil {
		return map[string]interface{}{"error": "task not found", "id": req.ID}
	}
	return task
}

// HandleUpdateTask — PUT /tasks/:id
// Partial update: only SET provided fields.
func HandleUpdateTask(body []byte) interface{} {
	db := GetDB()
	if db == nil {
		return map[string]interface{}{"error": "database not connected"}
	}

	var req struct {
		ID          string  `json:"id"`
		Title       *string `json:"title"`
		Description *string `json:"description"`
		Done        *bool   `json:"done"`
	}
	json.Unmarshal(body, &req)

	if req.Title != nil && *req.Title == "" {
		return map[string]interface{}{"error": "title must not be empty"}
	}

	if req.Title != nil {
		db.Exec("UPDATE tasks SET title = ?, updated_at = datetime('now') WHERE id = ?", *req.Title, req.ID)
	}
	if req.Description != nil {
		db.Exec("UPDATE tasks SET description = ?, updated_at = datetime('now') WHERE id = ?", *req.Description, req.ID)
	}
	if req.Done != nil {
		done := 0
		if *req.Done {
			done = 1
		}
		db.Exec("UPDATE tasks SET done = ?, updated_at = datetime('now') WHERE id = ?", done, req.ID)
	}

	return fetchTask(db, req.ID)
}

// HandleDeleteTask — DELETE /tasks/:id
// Delete by primary key, return the deleted task.
func HandleDeleteTask(body []byte) interface{} {
	db := GetDB()
	if db == nil {
		return map[string]interface{}{"error": "database not connected"}
	}

	var req struct {
		ID string `json:"id"`
	}
	json.Unmarshal(body, &req)

	task := fetchTask(db, req.ID)
	if task == nil {
		return map[string]interface{}{"error": "task not found", "id": req.ID}
	}

	db.Exec("DELETE FROM tasks WHERE id = ?", req.ID)
	return task
}

// HandleTaskStats — GET /tasks/stats
// Aggregate stats: total, done, pending.
func HandleTaskStats(body []byte) interface{} {
	db := GetDB()
	if db == nil {
		return map[string]interface{}{"error": "database not connected"}
	}

	var total, done int64
	db.QueryRow("SELECT COUNT(*) FROM tasks").Scan(&total)
	db.QueryRow("SELECT COUNT(*) FROM tasks WHERE done = 1").Scan(&done)

	return map[string]interface{}{
		"total":   total,
		"done":    done,
		"pending": total - done,
	}
}

func fetchTask(db *sql.DB, id string) map[string]interface{} {
	var tid, title, desc, createdAt, updatedAt string
	var done int64
	err := db.QueryRow("SELECT id, title, description, done, created_at, updated_at FROM tasks WHERE id = ?", id).
		Scan(&tid, &title, &desc, &done, &createdAt, &updatedAt)
	if err != nil {
		return nil
	}
	return map[string]interface{}{
		"id":          tid,
		"title":       title,
		"description": desc,
		"done":        done,
		"created_at":  createdAt,
		"updated_at":  updatedAt,
	}
}
