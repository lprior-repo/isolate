use std::{collections::HashMap, path::PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};

#[derive(Parser)]
#[command(name = "bead-kv")]
#[command(about = "Fast key-value bead tracker that cannot get overwhelmed")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(short, long, default_value = "~/.bead-kv")]
    db_path: PathBuf,
}

#[derive(Subcommand)]
enum Commands {
    /// List all beads, optionally filtered by status or labels
    List {
        /// Filter by status
        #[arg(short, long)]
        status: Option<String>,
        /// Filter by label (repeatable)
        #[arg(short, long)]
        label: Vec<String>,
        /// Output format
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Show details for a specific bead
    Show {
        /// Bead ID
        id: String,
    },
    /// Create a new bead
    Create {
        /// Bead ID
        id: String,
        /// Bead title
        #[arg(short, long)]
        title: String,
        /// Bead description
        #[arg(short, long)]
        description: String,
        /// Priority
        #[arg(short, long, default_value = "P2")]
        priority: String,
    },
    /// Update bead status
    Update {
        /// Bead ID
        id: String,
        /// New status
        #[arg(short, long)]
        status: Option<String>,
        /// Add labels (repeatable)
        #[arg(long)]
        add_label: Vec<String>,
        /// Remove labels (repeatable)
        #[arg(long)]
        remove_label: Vec<String>,
        /// Set actor
        #[arg(long)]
        actor: Option<String>,
    },
    /// Close a bead
    Close {
        /// Bead ID
        id: String,
        /// Reason
        #[arg(short, long)]
        reason: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Bead {
    id: String,
    title: String,
    description: String,
    status: String,
    labels: Vec<String>,
    priority: String,
    actor: Option<String>,
    created_at: i64,
    updated_at: i64,
    closed_at: Option<i64>,
    close_reason: Option<String>,
}

struct BeadKV {
    db: Db,
    tree: Tree,
}

impl BeadKV {
    fn new(path: PathBuf) -> Result<Self> {
        let path_str = path.to_string_lossy().to_string();
        let path = shellexpand::tilde(&path_str);
        let db = sled::open(path.as_ref())?;
        let tree = db.open_tree("beads")?;
        Ok(Self { db, tree })
    }

    fn get(&self, id: &str) -> Result<Option<Bead>> {
        Ok(self
            .tree
            .get(id)?
            .and_then(|v| serde_json::from_slice(&v).ok()))
    }

    fn set(&self, bead: &Bead) -> Result<()> {
        let key = bead.id.clone();
        let value = serde_json::to_vec(bead)?;
        self.tree.insert(key, value)?;
        self.db.flush()?;
        Ok(())
    }

    fn list(&self, status: Option<&str>, labels: &[String]) -> Result<Vec<Bead>> {
        let mut beads = Vec::new();

        for item in self.tree.iter() {
            let (_, v) = item?;
            if let Ok(bead) = serde_json::from_slice::<Bead>(&v) {
                // Status filter
                if let Some(s) = status {
                    if bead.status != s {
                        continue;
                    }
                }

                // Label filter (all must match)
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

    fn update<F>(&self, id: &str, updater: F) -> Result<()>
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    let kv = BeadKV::new(cli.db_path)?;

    match cli.command {
        Commands::List {
            status,
            label,
            format,
        } => {
            let beads = kv.list(status.as_deref(), &label)?;

            match format.as_str() {
                "json" => {
                    println!("{}", serde_json::to_string_pretty(&beads)?);
                }
                "table" => {
                    println!("{:<20} {:<10} {:<20} {}", "ID", "STATUS", "LABELS", "TITLE");
                    println!("{}", "-".repeat(80));
                    for bead in beads {
                        let labels = bead.labels.join(",");
                        println!(
                            "{:<20} {:<10} {:<20} {}",
                            bead.id, bead.status, labels, bead.title
                        );
                    }
                }
                _ => anyhow::bail!("Unknown format: {}", format),
            }
        }

        Commands::Show { id } => {
            if let Some(bead) = kv.get(&id)? {
                println!("{}", serde_json::to_string_pretty(&bead)?);
            } else {
                anyhow::bail!("Bead not found: {}", id);
            }
        }

        Commands::Create {
            id,
            title,
            description,
            priority,
        } => {
            let now = chrono::Utc::now().timestamp();
            let bead = Bead {
                id: id.clone(),
                title,
                description,
                status: "open".to_string(),
                labels: vec![],
                priority,
                actor: None,
                created_at: now,
                updated_at: now,
                closed_at: None,
                close_reason: None,
            };
            kv.set(&bead)?;
            println!("Created bead: {}", id);
        }

        Commands::Update {
            id,
            status,
            add_label,
            remove_label,
            actor,
        } => {
            kv.update(&id, |bead| {
                if let Some(s) = status {
                    bead.status = s;
                }
                for label in add_label {
                    if !bead.labels.contains(&label) {
                        bead.labels.push(label);
                    }
                }
                for label in remove_label {
                    bead.labels.retain(|l| l != &label);
                }
                if let Some(a) = actor {
                    bead.actor = Some(a);
                }
            })?;
            println!("Updated bead: {}", id);
        }

        Commands::Close { id, reason } => {
            kv.update(&id, |bead| {
                bead.status = "closed".to_string();
                bead.closed_at = Some(chrono::Utc::now().timestamp());
                bead.close_reason = Some(reason);
            })?;
            println!("Closed bead: {}", id);
        }
    }

    Ok(())
}
