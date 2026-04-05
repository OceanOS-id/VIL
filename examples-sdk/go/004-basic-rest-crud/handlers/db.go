package handlers

import (
	"crypto/rand"
	"database/sql"
	"fmt"
	"os"

	_ "github.com/mattn/go-sqlite3"
)

var db *sql.DB

func init() {
	dbURL := os.Getenv("DATABASE_URL")
	if dbURL == "" {
		dbURL = "tasks.db"
	}
	var err error
	db, err = sql.Open("sqlite3", dbURL)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to open database: %v\n", err)
		return
	}
	db.Exec(`CREATE TABLE IF NOT EXISTS tasks (
		id TEXT PRIMARY KEY,
		title TEXT NOT NULL,
		description TEXT DEFAULT '',
		done INTEGER DEFAULT 0,
		created_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
		updated_at TEXT DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))
	)`)
}

// GetDB returns the shared database connection.
func GetDB() *sql.DB {
	return db
}

// GenerateUUID generates a v4 UUID string.
func GenerateUUID() string {
	b := make([]byte, 16)
	rand.Read(b)
	b[6] = (b[6] & 0x0f) | 0x40
	b[8] = (b[8] & 0x3f) | 0x80
	return fmt.Sprintf("%08x-%04x-%04x-%04x-%012x",
		b[0:4], b[4:6], b[6:8], b[8:10], b[10:16])
}
