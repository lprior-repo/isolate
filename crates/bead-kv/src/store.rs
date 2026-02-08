use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bead {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub labels: Vec<String>,
    pub priority: String,
    pub actor: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub closed_at: Option<i64>,
    pub close_reason: Option<String>,
}

pub struct BeadKV {
    db: Db,
    tree: Tree,
}

impl BeadKV {
    pub fn new(path: PathBuf) -> Result<Self> {
        let expanded = shellexpand::tilde(&path.to_string_lossy());
        let db = sled::open(&expanded)?;
        let tree = db.open_tree("beads")?;
        Ok(Self { db, tree })
    }

    pub fn get(&self, id: &str) -> Result<Option<Bead>> {
        Ok(self
            .tree
            .get(id)?
            .and_then(|v| serde_json::from_slice(&v).ok()))
    }

    pub fn set(&self, bead: &Bead) -> Result<()> {
        let key = bead.id.clone();
        let value = serde_json::to_vec(bead)?;
        self.tree.insert(key, value)?;
        self.db.flush()?;
        Ok(())
    }

    pub fn list(&self, status: Option<&str>, labels: &[String]) -> Result<Vec<Bead>> {
        let mut beads = Vec::new();

        for item in self.tree.iter() {
            let (_, v) = item?;
            if let Ok(bead) = serde_json::from_slice::<Bead>(&v) {
                if let Some(s) = status {
                    if bead.status != s {
                        continue;
                    }
                }

                if !labels.is_empty() {
                    if !labels.iter().all(|l| bead.labels.contains(l)) {
                        continue;
                    }
                }

                beads.push(bead);
            }
        }

        Ok(beads)
    }

    pub fn update<F>(&self, id: &str, updater: F) -> Result<()>
    where
        F: FnOnce(&mut Bead),
    {
        if let Some(mut bead) = self.get(id)? {
            updater(&mut bead);
            bead.updated_at = chrono::Utc::now().timestamp();
            self.set(&bead)?;
        }
        Ok(())
    }
}
