//! State persistence for pipeline recovery

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use tracing::{debug, error, info};

use crate::state::{Pipeline, PipelineId};

/// Error types for state store operations
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Pipeline not found: {0}")]
    NotFound(String),
    #[error("Invalid state file: {0}")]
    InvalidState(String),
}

/// State store for persisting pipeline state
pub struct StateStore {
    /// Directory where state files are stored
    state_dir: PathBuf,
    /// In-memory cache of pipelines
    cache: HashMap<String, Pipeline>,
    /// Whether the cache is dirty
    dirty: bool,
}

impl StateStore {
    /// Create a new state store
    ///
    /// # Errors
    /// Returns an error if the directory cannot be created or state files cannot be loaded.
    pub fn new(state_dir: PathBuf) -> Result<Self, StoreError> {
        // Ensure directory exists
        fs::create_dir_all(&state_dir)?;

        let mut store = Self {
            state_dir,
            cache: HashMap::new(),
            dirty: false,
        };

        // Load existing state
        store.load_all()?;

        Ok(store)
    }

    /// Get the state file path for a pipeline
    fn state_file_path(&self, id: &PipelineId) -> PathBuf {
        self.state_dir.join(format!("{id}.json"))
    }

    /// Load all pipeline states from disk
    fn load_all(&mut self) -> Result<(), StoreError> {
        if !self.state_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&self.state_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match Self::load_single(&path) {
                    Ok(pipeline) => {
                        debug!("Loaded pipeline: {}", pipeline.id);
                        self.cache.insert(pipeline.id.0.clone(), pipeline);
                    }
                    Err(e) => {
                        error!("Failed to load pipeline from {:?}: {e}", path);
                    }
                }
            }
        }

        info!("Loaded {} pipelines from state store", self.cache.len());
        Ok(())
    }

    /// Load a single pipeline from a file
    fn load_single(path: &Path) -> Result<Pipeline, StoreError> {
        let content = fs::read_to_string(path)?;
        let pipeline: Pipeline =
            serde_json::from_str(&content).map_err(|e| StoreError::InvalidState(e.to_string()))?;
        Ok(pipeline)
    }

    /// Save a pipeline to disk
    fn save_single(&self, pipeline: &Pipeline) -> Result<(), StoreError> {
        let path = self.state_file_path(&pipeline.id);
        let content = serde_json::to_string_pretty(pipeline)?;
        fs::write(&path, content)?;
        debug!("Saved pipeline {} to {:?}", pipeline.id, path);
        Ok(())
    }

    /// Create a new pipeline
    ///
    /// # Errors
    /// Returns an error if the pipeline cannot be saved.
    pub fn create(&mut self, pipeline: Pipeline) -> Result<Pipeline, StoreError> {
        let id = pipeline.id.0.clone();
        self.save_single(&pipeline)?;
        self.cache.insert(id, pipeline.clone());
        self.dirty = true;
        Ok(pipeline)
    }

    /// Get a pipeline by ID
    ///
    /// # Errors
    /// Returns an error if the pipeline is not found.
    pub fn get(&self, id: &PipelineId) -> Result<&Pipeline, StoreError> {
        self.cache
            .get(&id.0)
            .ok_or_else(|| StoreError::NotFound(id.0.clone()))
    }

    /// Get a mutable pipeline by ID
    ///
    /// # Errors
    /// Returns an error if the pipeline is not found.
    pub fn get_mut(&mut self, id: &PipelineId) -> Result<&mut Pipeline, StoreError> {
        self.dirty = true;
        self.cache
            .get_mut(&id.0)
            .ok_or_else(|| StoreError::NotFound(id.0.clone()))
    }

    /// Update a pipeline
    ///
    /// # Errors
    /// Returns an error if the pipeline cannot be saved.
    pub fn update(&mut self, pipeline: Pipeline) -> Result<(), StoreError> {
        self.save_single(&pipeline)?;
        self.cache.insert(pipeline.id.0.clone(), pipeline);
        self.dirty = true;
        Ok(())
    }

    /// Delete a pipeline
    ///
    /// # Errors
    /// Returns an error if the pipeline is not found or cannot be deleted.
    pub fn delete(&mut self, id: &PipelineId) -> Result<(), StoreError> {
        let path = self.state_file_path(id);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        self.cache
            .remove(&id.0)
            .ok_or_else(|| StoreError::NotFound(id.0.clone()))?;
        self.dirty = true;
        Ok(())
    }

    /// List all pipelines
    #[must_use]
    pub fn list(&self) -> Vec<&Pipeline> {
        self.cache.values().collect()
    }

    /// List pipelines by state
    #[must_use]
    pub fn list_by_state(&self, state: crate::state::PipelineState) -> Vec<&Pipeline> {
        self.cache.values().filter(|p| p.state == state).collect()
    }

    /// Get pending pipelines that need recovery
    #[must_use]
    pub fn get_pending_recovery(&self) -> Vec<&Pipeline> {
        self.cache
            .values()
            .filter(|p| !p.state.is_terminal())
            .collect()
    }

    /// Check if a pipeline exists
    #[must_use]
    pub fn exists(&self, id: &PipelineId) -> bool {
        self.cache.contains_key(&id.0)
    }

    /// Force sync to disk
    ///
    /// # Errors
    /// Returns an error if sync fails.
    pub fn sync(&mut self) -> Result<(), StoreError> {
        if self.dirty {
            for pipeline in self.cache.values() {
                self.save_single(pipeline)?;
            }
            self.dirty = false;
            info!("Synced {} pipelines to disk", self.cache.len());
        }
        Ok(())
    }

    /// Export all state to a single JSON file
    ///
    /// # Errors
    /// Returns an error if export fails.
    pub fn export_all(&self, path: &Path) -> Result<(), StoreError> {
        let pipelines: Vec<&Pipeline> = self.cache.values().collect();
        let content = serde_json::to_string_pretty(&pipelines)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Import state from a JSON file
    ///
    /// # Errors
    /// Returns an error if import fails.
    pub fn import_from(&mut self, path: &Path) -> Result<usize, StoreError> {
        let content = fs::read_to_string(path)?;
        let pipelines: Vec<Pipeline> = serde_json::from_str(&content)?;

        let count = pipelines.len();
        for pipeline in pipelines {
            self.save_single(&pipeline)?;
            self.cache.insert(pipeline.id.0.clone(), pipeline);
        }

        self.dirty = true;
        info!("Imported {} pipelines from {:?}", count, path);
        Ok(count)
    }

    /// Clear all state (for testing)
    #[cfg(test)]
    pub fn clear(&mut self) -> Result<(), StoreError> {
        for id in self.cache.keys().cloned().collect::<Vec<_>>() {
            let path = self.state_file_path(&PipelineId(id));
            if path.exists() {
                fs::remove_file(path)?;
            }
        }
        self.cache.clear();
        self.dirty = false;
        Ok(())
    }
}

impl Drop for StateStore {
    fn drop(&mut self) {
        if let Err(e) = self.sync() {
            error!("Failed to sync state on drop: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::PipelineState;

    fn create_temp_store() -> (StateStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let store = StateStore::new(temp_dir.path().to_path_buf()).unwrap();
        (store, temp_dir)
    }

    #[test]
    fn test_create_and_get() {
        let (mut store, _temp) = create_temp_store();

        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        let id = pipeline.id.clone();

        store.create(pipeline).unwrap();

        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.spec_path, "specs/test.yaml");
    }

    #[test]
    fn test_update() {
        let (mut store, _temp) = create_temp_store();

        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        let id = pipeline.id.clone();

        store.create(pipeline).unwrap();

        let pipeline = store.get_mut(&id).unwrap();
        pipeline.transition_to(PipelineState::SpecReview).unwrap();
        let _ = pipeline;

        let retrieved = store.get(&id).unwrap();
        assert_eq!(retrieved.state, PipelineState::SpecReview);
    }

    #[test]
    fn test_delete() {
        let (mut store, _temp) = create_temp_store();

        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        let id = pipeline.id.clone();

        store.create(pipeline).unwrap();
        store.delete(&id).unwrap();

        assert!(store.get(&id).is_err());
    }

    #[test]
    fn test_list_by_state() {
        let (mut store, _temp) = create_temp_store();

        let p1 = Pipeline::new("specs/test1.yaml".to_string());
        let p2 = Pipeline::new("specs/test2.yaml".to_string());

        store.create(p1).unwrap();
        store.create(p2.clone()).unwrap();

        let p2_id = PipelineId(p2.id.0.clone());
        let pipeline = store.get_mut(&p2_id).unwrap();
        pipeline.transition_to(PipelineState::SpecReview).unwrap();

        let pending = store.list_by_state(PipelineState::Pending);
        assert_eq!(pending.len(), 1);
    }

    #[test]
    fn test_export_import() {
        let (mut store, _temp) = create_temp_store();

        let pipeline = Pipeline::new("specs/test.yaml".to_string());
        store.create(pipeline).unwrap();

        let export_path = _temp.path().join("export.json");
        store.export_all(&export_path).unwrap();

        let (mut store2, _temp2) = create_temp_store();
        store2.import_from(&export_path).unwrap();

        assert_eq!(store2.cache.len(), 1);
    }
}
