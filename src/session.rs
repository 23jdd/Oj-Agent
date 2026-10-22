use crate::{
    error::{AppError, Result},
    ir::*,
};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub stage: String,
    pub updated_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: String,
    pub session_id: String,
    pub kind: String,
    pub revision: i64,
    pub derived_from: Vec<String>,
    pub valid: bool,
    pub document: Value,
}
pub trait SessionStore {
    fn create(&mut self, input: &ProblemInput) -> Result<String>;
    fn list(&self) -> Result<Vec<SessionSummary>>;
    fn checkpoint(&mut self, session: &str, stage: &str, status: &str) -> Result<()>;
    fn add_artifact(
        &mut self,
        session: &str,
        kind: &str,
        document: &Value,
        parents: &[String],
        valid: bool,
    ) -> Result<String>;
    fn active(&self, session: &str, kind: &str) -> Result<Option<Artifact>>;
    fn delete(&mut self, session: &str) -> Result<()>;
}
pub struct SqliteStore {
    conn: Connection,
}
fn db(e: impl std::fmt::Display) -> AppError {
    AppError::io("SessionPersistence", e)
}
impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self> {
        Self::init(Connection::open(path).map_err(db)?)
    }
    pub fn memory() -> Result<Self> {
        Self::init(Connection::open_in_memory().map_err(db)?)
    }
    fn init(conn: Connection) -> Result<Self> {
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .map_err(db)?;
        if version > 1 {
            return Err(AppError::new(
                "SESSION_VERSION_UNSUPPORTED",
                "Configuration",
                "SessionPersistence",
                "This database was created by a newer application.",
            ));
        }
        conn.busy_timeout(std::time::Duration::from_secs(5))
            .map_err(db)?;
        conn.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS sessions(id TEXT PRIMARY KEY,title TEXT NOT NULL,input TEXT NOT NULL,status TEXT NOT NULL,stage TEXT NOT NULL,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS artifacts(id TEXT PRIMARY KEY,session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,kind TEXT NOT NULL,revision INTEGER NOT NULL,parents TEXT NOT NULL,valid INTEGER NOT NULL,document TEXT NOT NULL,UNIQUE(session_id,kind,revision));
CREATE TABLE IF NOT EXISTS active(session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,kind TEXT NOT NULL,artifact_id TEXT NOT NULL REFERENCES artifacts(id),PRIMARY KEY(session_id,kind));
CREATE TABLE IF NOT EXISTS events(session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,seq INTEGER NOT NULL,stage TEXT NOT NULL,status TEXT NOT NULL,created_at TEXT NOT NULL,PRIMARY KEY(session_id,seq));
CREATE TABLE IF NOT EXISTS errors(session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,document TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS cache(key TEXT PRIMARY KEY,document TEXT NOT NULL,created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS preferences(session_id TEXT PRIMARY KEY REFERENCES sessions(id) ON DELETE CASCADE,document TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS tool_calls(id TEXT PRIMARY KEY,session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,name TEXT NOT NULL,status TEXT NOT NULL,document TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS chat_messages(id TEXT PRIMARY KEY,session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,seq INTEGER NOT NULL,document TEXT NOT NULL,UNIQUE(session_id,seq));
PRAGMA user_version=1;
").map_err(db)?;
        Ok(Self { conn })
    }
    pub fn recover_interrupted(&mut self) -> Result<()> {
        self.conn
            .execute(
                "UPDATE sessions SET status='interrupted' WHERE status='processing'",
                [],
            )
            .map_err(db)?;
        Ok(())
    }
    pub fn create_chat(&mut self, title: &str) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO sessions VALUES(?1,?2,'{}','ready','Chat',?3,?3)",
                params![id, title, now],
            )
            .map_err(db)?;
        Ok(id)
    }
    pub fn append_message(&mut self, session: &str, message: &crate::chat::Message) -> Result<()> {
        let tx = self.conn.transaction().map_err(db)?;
        tx.execute("INSERT INTO chat_messages SELECT ?1,?2,COALESCE(MAX(seq),0)+1,?3 FROM chat_messages WHERE session_id=?2", params![message.id,session,serde_json::to_string(message).map_err(db)?]).map_err(db)?;
        tx.execute(
            "UPDATE sessions SET updated_at=?2 WHERE id=?1",
            params![session, Utc::now().to_rfc3339()],
        )
        .map_err(db)?;
        tx.commit().map_err(db)
    }
    pub fn messages(&self, session: &str) -> Result<Vec<crate::chat::Message>> {
        let mut query = self
            .conn
            .prepare("SELECT document FROM chat_messages WHERE session_id=?1 ORDER BY seq")
            .map_err(db)?;
        query
            .query_map([session], |row| row.get::<_, String>(0))
            .map_err(db)?
            .map(|row| serde_json::from_str(&row.map_err(db)?).map_err(db))
            .collect()
    }
    pub fn chat_animation(&self, session: &str, id: &str) -> Result<crate::chat::AnimationBundle> {
        let value: String = self.conn.query_row("SELECT document FROM artifacts WHERE session_id=?1 AND id=?2 AND kind='chat-animation' AND valid=1", params![session,id], |r| r.get(0)).optional().map_err(db)?.ok_or_else(|| AppError::new("ANIMATION_NOT_FOUND", "Input", "Chat", "This animation is not available in the current conversation."))?;
        serde_json::from_str(&value).map_err(db)
    }
    pub fn cache_get(&self, key: &str) -> Result<Option<Value>> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT document FROM cache WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()
            .map_err(db)?;
        raw.map(|text| serde_json::from_str(&text).map_err(db))
            .transpose()
    }
    pub fn cache_put(&mut self, key: &str, document: &Value) -> Result<()> {
        self.conn.execute("INSERT INTO cache VALUES(?1,?2,?3) ON CONFLICT(key) DO UPDATE SET document=excluded.document,created_at=excluded.created_at",params![key,serde_json::to_string(document).map_err(db)?,Utc::now().to_rfc3339()]).map_err(db)?;
        Ok(())
    }
    pub fn preferences(&self, id: &str) -> Result<Value> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT document FROM preferences WHERE session_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)?;
        raw.map(|text| serde_json::from_str(&text).map_err(db))
            .transpose()
            .map(|v| v.unwrap_or_else(|| serde_json::json!({})))
    }
    pub fn save_preferences(&mut self, id: &str, value: &Value) -> Result<()> {
        self.conn.execute("INSERT INTO preferences VALUES(?1,?2) ON CONFLICT(session_id) DO UPDATE SET document=excluded.document",params![id,serde_json::to_string(value).map_err(db)?]).map_err(db)?;
        Ok(())
    }
    pub fn activate(&mut self, id: &str, artifact_id: &str) -> Result<()> {
        let artifact = self
            .artifacts(id)?
            .into_iter()
            .find(|a| a.id == artifact_id && a.valid)
            .ok_or_else(|| {
                AppError::validation("SessionPersistence", "Unknown or invalid revision.")
            })?;
        self.add_artifact(
            id,
            &artifact.kind,
            &artifact.document,
            &artifact.derived_from,
            true,
        )?;
        Ok(())
    }
    pub fn record_tool(
        &mut self,
        id: &str,
        name: &str,
        status: &str,
        document: &Value,
    ) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO tool_calls VALUES(?1,?2,?3,?4,?5)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    id,
                    name,
                    status,
                    serde_json::to_string(document).map_err(db)?
                ],
            )
            .map_err(db)?;
        Ok(())
    }
    pub fn update_input(&mut self, id: &str, input: &ProblemInput) -> Result<()> {
        input.validate()?;
        self.conn
            .execute(
                "UPDATE sessions SET input=?2,title=?3 WHERE id=?1",
                params![id, serde_json::to_string(input).map_err(db)?, input.title],
            )
            .map_err(db)?;
        Ok(())
    }
    pub fn clear_active(&mut self, id: &str, kind: &str) -> Result<()> {
        self.conn
            .execute(
                "DELETE FROM active WHERE session_id=?1 AND kind=?2",
                params![id, kind],
            )
            .map_err(db)?;
        Ok(())
    }
    pub fn input(&self, id: &str) -> Result<ProblemInput> {
        let raw: String = self
            .conn
            .query_row("SELECT input FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .map_err(db)?;
        serde_json::from_str(&raw).map_err(db)
    }
    pub fn record_error(&mut self, id: &str, error: &AppError) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO errors(session_id,document) VALUES(?1,?2)",
                params![id, serde_json::to_string(error).map_err(db)?],
            )
            .map_err(db)?;
        self.checkpoint(
            id,
            &error.stage,
            if error.kind == "Cancelled" {
                "cancelled"
            } else {
                "failed"
            },
        )
    }
    pub fn record_export(&mut self, id: &str, path: &Path, format: &str) -> Result<String> {
        let animation = self.active(id, "animation")?.ok_or_else(|| {
            AppError::validation(
                "Export",
                "No active animation to associate with this export.",
            )
        })?;
        let path = path.canonicalize().map_err(|e| AppError::io("Export", e))?;
        self.add_artifact(
            id,
            "export",
            &serde_json::json!({"path":path,"format":format}),
            &[animation.id],
            true,
        )
    }
    pub fn artifacts(&self, id: &str) -> Result<Vec<Artifact>> {
        let mut query=self.conn.prepare("SELECT id,kind,revision,parents,valid,document FROM artifacts WHERE session_id=?1 ORDER BY rowid").map_err(db)?;
        let rows = query
            .query_map([id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, bool>(4)?,
                    r.get::<_, String>(5)?,
                ))
            })
            .map_err(db)?;
        rows.map(|row| {
            let (a, k, r, p, v, d) = row.map_err(db)?;
            Ok(Artifact {
                id: a,
                session_id: id.into(),
                kind: k,
                revision: r,
                derived_from: serde_json::from_str(&p).map_err(db)?,
                valid: v,
                document: serde_json::from_str(&d).map_err(db)?,
            })
        })
        .collect()
    }
}
impl SessionStore for SqliteStore {
    fn create(&mut self, input: &ProblemInput) -> Result<String> {
        input.validate()?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO sessions VALUES(?1,?2,?3,'created','InputNormalization',?4,?4)",
                params![
                    id,
                    input.title,
                    serde_json::to_string(input).map_err(db)?,
                    now
                ],
            )
            .map_err(db)?;
        Ok(id)
    }
    fn list(&self) -> Result<Vec<SessionSummary>> {
        let mut q = self
            .conn
            .prepare(
                "SELECT id,title,status,stage,updated_at FROM sessions ORDER BY updated_at DESC",
            )
            .map_err(db)?;
        q.query_map([], |r| {
            Ok(SessionSummary {
                id: r.get(0)?,
                title: r.get(1)?,
                status: r.get(2)?,
                stage: r.get(3)?,
                updated_at: r.get(4)?,
            })
        })
        .map_err(db)?
        .map(|r| r.map_err(db))
        .collect()
    }
    fn checkpoint(&mut self, id: &str, stage: &str, status: &str) -> Result<()> {
        let tx = self.conn.transaction().map_err(db)?;
        let now = Utc::now().to_rfc3339();
        if tx
            .execute(
                "UPDATE sessions SET status=?2,stage=?3,updated_at=?4 WHERE id=?1",
                params![id, status, stage, now],
            )
            .map_err(db)?
            == 0
        {
            return Err(AppError::new(
                "SESSION_NOT_FOUND",
                "Input",
                "SessionPersistence",
                "Unknown session.",
            ));
        }
        tx.execute("INSERT INTO events SELECT ?1,COALESCE(MAX(seq),0)+1,?2,?3,?4 FROM events WHERE session_id=?1",params![id,stage,status,now]).map_err(db)?;
        tx.commit().map_err(db)
    }
    fn add_artifact(
        &mut self,
        id: &str,
        kind: &str,
        document: &Value,
        parents: &[String],
        valid: bool,
    ) -> Result<String> {
        let tx = self.conn.transaction().map_err(db)?;
        for p in parents {
            let count: i64 = tx
                .query_row(
                    "SELECT COUNT(*) FROM artifacts WHERE id=?1 AND session_id=?2 AND valid=1",
                    params![p, id],
                    |r| r.get(0),
                )
                .map_err(db)?;
            if count != 1 {
                return Err(AppError::validation(
                    "SessionPersistence",
                    "Artifact parent is invalid or belongs to another session.",
                ));
            }
        }
        let revision: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(revision),0)+1 FROM artifacts WHERE session_id=?1 AND kind=?2",
                params![id, kind],
                |r| r.get(0),
            )
            .map_err(db)?;
        let artifact = uuid::Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO artifacts VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                artifact,
                id,
                kind,
                revision,
                serde_json::to_string(parents).map_err(db)?,
                valid,
                serde_json::to_string(document).map_err(db)?
            ],
        )
        .map_err(db)?;
        if valid {
            // Replacing an upstream artifact invalidates active downstream references, never history.
            let downstream: &[&str] = match kind {
                "input" => &[
                    "analysis",
                    "algorithm",
                    "trace",
                    "visualization",
                    "animation",
                    "export",
                ],
                "algorithm" => &["trace", "visualization", "animation", "export"],
                "trace" => &["visualization", "animation", "export"],
                "visualization" => &["animation", "export"],
                "animation" => &["export"],
                _ => &[],
            };
            for k in downstream {
                tx.execute(
                    "DELETE FROM active WHERE session_id=?1 AND kind=?2",
                    params![id, k],
                )
                .map_err(db)?;
            }
            tx.execute("INSERT INTO active VALUES(?1,?2,?3) ON CONFLICT(session_id,kind) DO UPDATE SET artifact_id=excluded.artifact_id",params![id,kind,artifact]).map_err(db)?;
        }
        tx.commit().map_err(db)?;
        Ok(artifact)
    }
    fn active(&self, id: &str, kind: &str) -> Result<Option<Artifact>> {
        let raw: Option<String> = self
            .conn
            .query_row(
                "SELECT artifact_id FROM active WHERE session_id=?1 AND kind=?2",
                params![id, kind],
                |r| r.get(0),
            )
            .optional()
            .map_err(db)?;
        match raw {
            Some(a) => Ok(self.artifacts(id)?.into_iter().find(|item| item.id == a)),
            None => Ok(None),
        }
    }
    fn delete(&mut self, id: &str) -> Result<()> {
        self.conn
            .execute("DELETE FROM sessions WHERE id=?1", [id])
            .map_err(db)?;
        Ok(())
    }
}
