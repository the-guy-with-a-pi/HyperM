use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppRecord {
    pub name: String,
    pub pid: u32,
    pub status: String,
    pub memory_mb: u32,
    pub cpus: u8,
    pub guest_command: Vec<String>,
    pub stdout_log: PathBuf,
    pub stderr_log: PathBuf,
}

impl AppRecord {
    pub fn from_launch(launch: &crate::runtime::VmLaunch, pid: u32) -> Self {
        Self {
            name: launch.name.clone(),
            pid,
            status: "online".into(),
            memory_mb: launch.resources.memory_mb,
            cpus: launch.resources.cpus,
            guest_command: launch.guest_command.clone(),
            stdout_log: launch.app_dir.join("stdout.log"),
            stderr_log: launch.app_dir.join("stderr.log"),
        }
    }
}

pub struct StateStore {
    root: PathBuf,
    state_path: PathBuf,
}

impl StateStore {
    pub fn new(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join("apps"))?;
        let state_path = root.join("state.json");
        if !state_path.exists() {
            fs::write(&state_path, b"[]")?;
        }
        Ok(Self { root, state_path })
    }

    pub fn app_dir(&self, name: &str) -> Result<PathBuf> {
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            bail!("app name must contain only letters, numbers, '-' or '_'")
        }
        Ok(self.root.join("apps").join(name))
    }

    pub fn list(&self) -> Result<Vec<AppRecord>> {
        Ok(serde_json::from_slice(&fs::read(&self.state_path)?)?)
    }

    pub fn get(&self, name: &str) -> Result<Option<AppRecord>> {
        Ok(self.list()?.into_iter().find(|app| app.name == name))
    }

    pub fn require(&self, name: &str) -> Result<AppRecord> {
        self.get(name)?.ok_or_else(|| anyhow::anyhow!("unknown app '{name}'"))
    }

    pub fn insert(&self, app: AppRecord) -> Result<()> {
        let mut apps = self.list()?;
        apps.push(app);
        self.write(apps)
    }

    pub fn update(&self, app: AppRecord) -> Result<()> {
        let mut apps = self.list()?;
        let existing = apps.iter_mut().find(|current| current.name == app.name).ok_or_else(|| anyhow::anyhow!("unknown app '{}'", app.name))?;
        *existing = app;
        self.write(apps)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        self.write(self.list()?.into_iter().filter(|app| app.name != name).collect())
    }

    fn write(&self, apps: Vec<AppRecord>) -> Result<()> {
        let temporary = self.state_path.with_extension("tmp");
        fs::write(&temporary, serde_json::to_vec_pretty(&apps)?)?;
        fs::rename(temporary, &self.state_path)?;
        Ok(())
    }
}
