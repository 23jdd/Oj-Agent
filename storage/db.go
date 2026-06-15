package storage

import (
	"database/sql"
	"fmt"
	"log"

	_ "modernc.org/sqlite"
)

type DB struct {
	conn *sql.DB
}

func Open(path string) (*DB, error) {
	conn, err := sql.Open("sqlite", path)
	if err != nil {
		return nil, fmt.Errorf("open db: %w", err)
	}

	conn.SetMaxOpenConns(1)

	if err := conn.Ping(); err != nil {
		conn.Close()
		return nil, fmt.Errorf("ping db: %w", err)
	}

	db := &DB{conn: conn}
	if err := db.migrate(); err != nil {
		conn.Close()
		return nil, fmt.Errorf("migrate: %w", err)
	}

	log.Printf("[storage] opened %s", path)
	return db, nil
}

func (db *DB) Close() error {
	return db.conn.Close()
}

func (db *DB) migrate() error {
	queries := []string{
		`PRAGMA journal_mode=WAL`,
		`PRAGMA foreign_keys=ON`,
		`CREATE TABLE IF NOT EXISTS sessions (
			id TEXT PRIMARY KEY,
			title TEXT NOT NULL DEFAULT '',
			created_at TEXT NOT NULL DEFAULT '',
			updated_at TEXT NOT NULL DEFAULT ''
		)`,
		`CREATE TABLE IF NOT EXISTS messages (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
			role TEXT NOT NULL DEFAULT '',
			content TEXT NOT NULL DEFAULT '',
			time TEXT NOT NULL DEFAULT ''
		)`,
		`CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id)`,
	}

	for _, q := range queries {
		if _, err := db.conn.Exec(q); err != nil {
			return fmt.Errorf("exec %q: %w", q, err)
		}
	}

	return nil
}

type SessionRow struct {
	ID        string
	Title     string
	CreatedAt string
	UpdatedAt string
}

type MessageRow struct {
	ID        int64
	SessionID string
	Role      string
	Content   string
	Time      string
}

func (db *DB) InsertSession(s *SessionRow) error {
	_, err := db.conn.Exec(
		`INSERT OR REPLACE INTO sessions (id, title, created_at, updated_at) VALUES (?, ?, ?, ?)`,
		s.ID, s.Title, s.CreatedAt, s.UpdatedAt,
	)
	return err
}

func (db *DB) UpdateSession(id, title, updatedAt string) error {
	_, err := db.conn.Exec(
		`UPDATE sessions SET title = ?, updated_at = ? WHERE id = ?`,
		title, updatedAt, id,
	)
	return err
}

func (db *DB) DeleteSession(id string) error {
	tx, err := db.conn.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()

	if _, err := tx.Exec(`DELETE FROM messages WHERE session_id = ?`, id); err != nil {
		return err
	}
	if _, err := tx.Exec(`DELETE FROM sessions WHERE id = ?`, id); err != nil {
		return err
	}
	return tx.Commit()
}

func (db *DB) ListSessions() ([]SessionRow, error) {
	rows, err := db.conn.Query(`SELECT id, title, created_at, updated_at FROM sessions ORDER BY updated_at DESC`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var result []SessionRow
	for rows.Next() {
		var s SessionRow
		if err := rows.Scan(&s.ID, &s.Title, &s.CreatedAt, &s.UpdatedAt); err != nil {
			return nil, err
		}
		result = append(result, s)
	}
	return result, rows.Err()
}

func (db *DB) GetSession(id string) (*SessionRow, error) {
	row := db.conn.QueryRow(`SELECT id, title, created_at, updated_at FROM sessions WHERE id = ?`, id)
	var s SessionRow
	if err := row.Scan(&s.ID, &s.Title, &s.CreatedAt, &s.UpdatedAt); err != nil {
		if err == sql.ErrNoRows {
			return nil, nil
		}
		return nil, err
	}
	return &s, nil
}

func (db *DB) InsertMessage(sessionID string, m *MessageRow) error {
	_, err := db.conn.Exec(
		`INSERT INTO messages (session_id, role, content, time) VALUES (?, ?, ?, ?)`,
		sessionID, m.Role, m.Content, m.Time,
	)
	return err
}

func (db *DB) GetMessages(sessionID string) ([]MessageRow, error) {
	rows, err := db.conn.Query(
		`SELECT id, session_id, role, content, time FROM messages WHERE session_id = ? ORDER BY id ASC`,
		sessionID,
	)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var result []MessageRow
	for rows.Next() {
		var m MessageRow
		if err := rows.Scan(&m.ID, &m.SessionID, &m.Role, &m.Content, &m.Time); err != nil {
			return nil, err
		}
		result = append(result, m)
	}
	return result, rows.Err()
}

func (db *DB) DeleteMessages(sessionID string) error {
	_, err := db.conn.Exec(`DELETE FROM messages WHERE session_id = ?`, sessionID)
	return err
}
