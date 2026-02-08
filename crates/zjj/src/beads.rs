//! Unified bead repository for managing issues in both `SQLite` and JSONL formats

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tokio::fs;

/// Status of an issue in the beads tracker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BeadStatus {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed,
}

impl std::fmt::Display for BeadStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Deferred => "deferred",
            Self::Closed => "closed",
        };
        write!(f, "{s}")
    }
}

impl std::str::FromStr for BeadStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" | "●" | "ready" => Ok(Self::Open),
            "in_progress" | "working" | "in-progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "deferred" => Ok(Self::Deferred),
            "closed" | "completed" | "done" => Ok(Self::Closed),
            _ => Err(anyhow::anyhow!("Invalid bead status: {s}")),
        }
    }
}

/// Metadata for a bead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadMetadata {
    pub id: String,
    pub title: String,
    pub status: BeadStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Enable `WAL` mode on the `SQLite` connection for better crash recovery.
///
/// # Errors
///
/// Returns `Error` if the `PRAGMA` statement fails.
async fn enable_wal_mode(pool: &SqlitePool) -> Result<()> {
    sqlx::query("PRAGMA journal_mode=WAL;")
        .execute(pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to enable WAL mode: {e}"))?;
    Ok(())
}

/// Unified repository for beads
pub struct BeadRepository {
    root: PathBuf,
}

impl BeadRepository {
    /// Create a new repository instance
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Get bead by ID
    pub async fn get_bead(&self, id: &str) -> Result<Option<BeadMetadata>> {
        // Railway-oriented: Try SQLite, fallback to JSONL
        match self.get_bead_sqlite(id).await {
            Ok(Some(bead)) => Ok(Some(bead)),
            _ => self.get_bead_jsonl(id).await,
        }
    }

    /// Update bead status
    pub async fn update_status(&self, id: &str, status: BeadStatus) -> Result<()> {
        if self.beads_db_path().exists() {
            self.update_status_sqlite(id, status).await?;
        } else if self.issues_jsonl_path().exists() {
            self.update_status_jsonl(id, status).await?;
        } else {
            anyhow::bail!("No beads database or issues file found to update");
        }

        Ok(())
    }

    /// List all beads
    pub async fn list_beads(&self) -> Result<Vec<BeadMetadata>> {
        // Load from JSONL then supplement with SQLite using im::HashMap for functional merging
        let jsonl_beads = self
            .list_beads_jsonl()
            .await
            .unwrap_or_else(|_| Vec::new());
        let initial_map = jsonl_beads
            .into_iter()
            .fold(im::HashMap::new(), |mut acc, b| {
                acc.insert(b.id.clone(), b);
                acc
            });

        let sqlite_beads = self
            .list_beads_sqlite()
            .await
            .unwrap_or_else(|_| Vec::new());
        let final_map = sqlite_beads.into_iter().fold(initial_map, |mut acc, b| {
            acc.insert(b.id.clone(), b);
            acc
        });

        Ok(final_map.into_iter().map(|(_, v)| v).collect())
    }

    async fn list_beads_sqlite(&self) -> Result<Vec<BeadMetadata>> {
        let path = self.beads_db_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let connection_string = format!("sqlite:{}?mode=rw", path.display());
        let pool = SqlitePool::connect(&connection_string).await?;

        // Enable WAL mode for better crash recovery
        enable_wal_mode(&pool).await?;

        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT id, title, status FROM issues")
                .fetch_all(&pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|(id, title, status_str)| BeadMetadata {
                id,
                title,
                status: status_str.parse().map_or(BeadStatus::Open, |s| s),
                description: None,
            })
            .collect())
    }

    async fn list_beads_jsonl(&self) -> Result<Vec<BeadMetadata>> {
        let path = self.issues_jsonl_path();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(path).await?;

        let beads = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| {
                serde_json::from_str::<serde_json::Value>(line)
                    .ok()
                    .and_then(|json| {
                        let id = json.get("id")?.as_str()?;
                        let title = json
                            .get("title")
                            .and_then(|v| v.as_str())
                            .map_or_else(|| "Unknown".to_string(), |s| s.to_string());
                        let status_str = json
                            .get("status")
                            .and_then(|v| v.as_str())
                            .map_or("open", |s| s);
                        let description = json
                            .get("description")
                            .and_then(|v| v.as_str())
                            .map(String::from);

                        Some(BeadMetadata {
                            id: id.to_string(),
                            title,
                            status: status_str.parse().map_or(BeadStatus::Open, |s| s),
                            description,
                        })
                    })
            })
            .collect();

        Ok(beads)
    }

    fn beads_db_path(&self) -> PathBuf {
        self.root.join(".beads/beads.db")
    }

    fn issues_jsonl_path(&self) -> PathBuf {
        self.root.join(".beads/issues.jsonl")
    }

    async fn get_bead_sqlite(&self, id: &str) -> Result<Option<BeadMetadata>> {
        let path = self.beads_db_path();
        if !path.exists() {
            return Ok(None);
        }

        let connection_string = format!("sqlite:{}?mode=rw", path.display());
        let pool = SqlitePool::connect(&connection_string).await?;

        // Enable WAL mode for better crash recovery
        enable_wal_mode(&pool).await?;

        let result: Option<(String, String, String)> =
            sqlx::query_as("SELECT id, title, status FROM issues WHERE id = ?1")
                .bind(id)
                .fetch_optional(&pool)
                .await?;

        Ok(result.map(|(id, title, status_str)| BeadMetadata {
            id,
            title,
            status: status_str.parse().map_or(BeadStatus::Open, |s| s),
            description: None,
        }))
    }

    async fn get_bead_jsonl(&self, id: &str) -> Result<Option<BeadMetadata>> {
        let path = self.issues_jsonl_path();
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(path).await?;

        let bead = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .find_map(|line| {
                let json: serde_json::Value = serde_json::from_str(line).ok()?;
                if json.get("id").and_then(|v| v.as_str()) == Some(id) {
                    let title = json
                        .get("title")
                        .and_then(|v| v.as_str())
                        .map_or_else(|| "Unknown".to_string(), |s| s.to_string());
                    let status_str = json
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map_or("open", |s| s);
                    let description = json
                        .get("description")
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    Some(BeadMetadata {
                        id: id.to_string(),
                        title,
                        status: status_str.parse().map_or(BeadStatus::Open, |s| s),
                        description,
                    })
                } else {
                    None
                }
            });

        Ok(bead)
    }

    async fn update_status_sqlite(&self, id: &str, status: BeadStatus) -> Result<()> {
        let path = self.beads_db_path();
        let connection_string = format!("sqlite:{}?mode=rw", path.display());
        let pool = SqlitePool::connect(&connection_string).await?;

        // Enable WAL mode for better crash recovery
        enable_wal_mode(&pool).await?;

        sqlx::query("UPDATE issues SET status = ?1, updated_at = datetime('now') WHERE id = ?2")
            .bind(status.to_string())
            .bind(id)
            .execute(&pool)
            .await?;

        Ok(())
    }

    async fn update_status_jsonl(&self, id: &str, status: BeadStatus) -> Result<()> {
        let path = self.issues_jsonl_path();
        let content = fs::read_to_string(&path).await?;

        let (new_content, updated) = content.lines().filter(|l| !l.trim().is_empty()).try_fold(
            (String::new(), false),
            |(mut acc, mut updated), line| {
                let mut json: serde_json::Value = serde_json::from_str(line)?;
                if json.get("id").and_then(|v| v.as_str()) == Some(id) {
                    json["status"] = serde_json::json!(status.to_string());
                    updated = true;
                }
                acc.push_str(&json.to_string());
                acc.push('\n');
                Ok::<(String, bool), anyhow::Error>((acc, updated))
            },
        )?;

        if updated {
            fs::write(path, new_content).await?;
        }

        Ok(())
    }
}
