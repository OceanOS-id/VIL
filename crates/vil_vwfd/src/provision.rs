// =============================================================================
// vil_vwfd::provision — Versioned Workflow Registry with Multi-Tenancy
// =============================================================================
//
// Enabled via `.provision(true)` on VwfdApp builder.
//
// Features:
//   - Upload YAML → compile → register route (POST /api/admin/upload)
//   - Versioning: each upload = new revision, blue-green activation
//   - Multi-tenant: namespace routing via X-Tenant-Id header
//   - Activate/Deactivate per workflow
//   - Persist YAML to disk for restart recovery

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::Serialize;
use crate::graph::VilwGraph;
use crate::compiler::compile;
use crate::handler::WorkflowRouter;

/// Workflow version entry.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowVersion {
    pub id: String,
    pub tenant: String,
    pub revision: u32,
    pub node_count: usize,
    pub webhook_path: Option<String>,
    pub active: bool,
    pub uploaded_at: u64,
}

/// All versions of a single workflow.
struct WorkflowSlot {
    versions: HashMap<u32, (Arc<VilwGraph>, WorkflowVersion, Vec<u8>)>,
    active_revision: u32,
    webhook_path: Option<String>,
}

/// Versioned workflow registry with multi-tenancy.
pub struct WorkflowRegistry {
    slots: RwLock<HashMap<String, WorkflowSlot>>,  // key = "{tenant}/{id}"
    workflow_dir: String,
}

impl WorkflowRegistry {
    pub fn new(workflow_dir: &str) -> Self {
        Self {
            slots: RwLock::new(HashMap::new()),
            workflow_dir: workflow_dir.to_string(),
        }
    }

    /// Upload YAML → compile → register. Returns new version entry.
    pub fn upload(&self, tenant: &str, yaml: &str) -> Result<WorkflowVersion, String> {
        let graph = compile(yaml).map_err(|e| format!("compile: {:?}", e))?;
        let id = graph.id.clone();
        let webhook_path = graph.webhook_route.clone();
        let node_count = graph.nodes.len();
        let ns = ns_key(tenant, &id);

        let mut slots = self.slots.write().unwrap();
        let slot = slots.entry(ns.clone()).or_insert_with(|| WorkflowSlot {
            versions: HashMap::new(),
            active_revision: 0,
            webhook_path: webhook_path.clone(),
        });

        let revision = slot.versions.len() as u32 + 1;
        let is_first = slot.versions.is_empty();

        let entry = WorkflowVersion {
            id: id.clone(),
            tenant: tenant.to_string(),
            revision,
            node_count,
            webhook_path: webhook_path.clone(),
            active: is_first,
            uploaded_at: now_ms(),
        };

        slot.versions.insert(revision, (Arc::new(graph), entry.clone(), yaml.as_bytes().to_vec()));
        slot.webhook_path = webhook_path;

        if is_first {
            slot.active_revision = revision;
        }

        // Persist YAML to disk
        let dir = format!("{}/{}", self.workflow_dir, tenant);
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::write(format!("{}/{}.v{}.yaml", dir, id, revision), yaml);

        Ok(entry)
    }

    /// Activate a specific revision (blue-green).
    pub fn activate(&self, tenant: &str, workflow_id: &str, revision: u32) -> Result<WorkflowVersion, String> {
        let ns = ns_key(tenant, workflow_id);
        let mut slots = self.slots.write().unwrap();
        let slot = slots.get_mut(&ns)
            .ok_or_else(|| format!("workflow '{}' not found for tenant '{}'", workflow_id, tenant))?;

        if !slot.versions.contains_key(&revision) {
            return Err(format!("revision {} not found", revision));
        }

        slot.active_revision = revision;
        for (v, (_, entry, _)) in slot.versions.iter_mut() {
            entry.active = *v == revision;
        }

        let (_, entry, _) = slot.versions.get(&revision).unwrap();
        Ok(entry.clone())
    }

    /// Deactivate workflow — stop serving, keep in registry.
    pub fn deactivate(&self, tenant: &str, workflow_id: &str) -> Result<(), String> {
        let ns = ns_key(tenant, workflow_id);
        let mut slots = self.slots.write().unwrap();
        let slot = slots.get_mut(&ns)
            .ok_or_else(|| format!("workflow '{}' not found", workflow_id))?;

        for (_, (_, entry, _)) in slot.versions.iter_mut() {
            entry.active = false;
        }
        Ok(())
    }

    /// Remove workflow entirely.
    pub fn remove(&self, tenant: &str, workflow_id: &str) -> bool {
        let ns = ns_key(tenant, workflow_id);
        let mut slots = self.slots.write().unwrap();
        if slots.remove(&ns).is_some() {
            // Delete persisted files
            let dir = format!("{}/{}", self.workflow_dir, tenant);
            if let Ok(entries) = std::fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with(&format!("{}.", workflow_id)) {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Get active graph for routing.
    pub fn get_active(&self, tenant: &str, workflow_id: &str) -> Option<Arc<VilwGraph>> {
        let ns = ns_key(tenant, workflow_id);
        let slots = self.slots.read().unwrap();
        let slot = slots.get(&ns)?;
        slot.versions.get(&slot.active_revision).map(|(g, _, _)| g.clone())
    }

    /// Get active graph by webhook path (for HTTP routing).
    pub fn get_by_path(&self, tenant: &str, path: &str) -> Option<Arc<VilwGraph>> {
        let slots = self.slots.read().unwrap();
        for (ns, slot) in slots.iter() {
            if !ns.starts_with(&format!("{}/", tenant)) { continue; }
            if let Some(ref wp) = slot.webhook_path {
                if wp == path {
                    let (_, entry, _) = slot.versions.get(&slot.active_revision)?;
                    if entry.active {
                        return slot.versions.get(&slot.active_revision).map(|(g, _, _)| g.clone());
                    }
                }
            }
        }
        None
    }

    /// List all workflows.
    pub fn list(&self, tenant: Option<&str>) -> Vec<WorkflowVersion> {
        let slots = self.slots.read().unwrap();
        let mut result = Vec::new();
        for (ns, slot) in slots.iter() {
            if let Some(t) = tenant {
                if !ns.starts_with(&format!("{}/", t)) { continue; }
            }
            for (_, (_, entry, _)) in &slot.versions {
                result.push(entry.clone());
            }
        }
        result
    }

    /// Get workflow status.
    pub fn status(&self, tenant: &str, workflow_id: &str) -> Option<serde_json::Value> {
        let ns = ns_key(tenant, workflow_id);
        let slots = self.slots.read().unwrap();
        let slot = slots.get(&ns)?;

        let versions: Vec<serde_json::Value> = slot.versions.iter()
            .map(|(v, (g, entry, _))| serde_json::json!({
                "revision": v,
                "active": entry.active,
                "node_count": g.nodes.len(),
                "uploaded_at": entry.uploaded_at,
            }))
            .collect();

        Some(serde_json::json!({
            "workflow_id": workflow_id,
            "tenant": tenant,
            "active_revision": slot.active_revision,
            "webhook_path": slot.webhook_path,
            "versions": versions,
        }))
    }

    /// Sync to WorkflowRouter (call after upload/activate/deactivate).
    pub fn sync_router(&self, router: &WorkflowRouter) {
        router.clear();
        let slots = self.slots.read().unwrap();
        for (_, slot) in slots.iter() {
            if let Some((graph, entry, _)) = slot.versions.get(&slot.active_revision) {
                if !entry.active { continue; }
                if let Some(ref path) = slot.webhook_path {
                    let method = graph.webhook_method.clone();
                    router.register(method, path.clone(), graph.clone());
                }
            }
        }
    }

    /// Load all YAML from workflow directory on startup.
    pub fn load_from_dir(&self) -> (usize, Vec<String>) {
        let mut loaded = 0;
        let mut errors = Vec::new();

        let Ok(tenants) = std::fs::read_dir(&self.workflow_dir) else {
            return (0, vec![format!("cannot read dir: {}", self.workflow_dir)]);
        };

        for tenant_entry in tenants.flatten() {
            if !tenant_entry.path().is_dir() {
                // Root-level YAML = _default tenant
                if let Some(ext) = tenant_entry.path().extension() {
                    if ext == "yaml" || ext == "yml" {
                        match std::fs::read_to_string(tenant_entry.path()) {
                            Ok(yaml) => match self.upload("_default", &yaml) {
                                Ok(_) => loaded += 1,
                                Err(e) => errors.push(format!("{}: {}", tenant_entry.path().display(), e)),
                            },
                            Err(e) => errors.push(format!("{}: {}", tenant_entry.path().display(), e)),
                        }
                    }
                }
                continue;
            }

            let tenant = tenant_entry.file_name().to_string_lossy().to_string();
            let Ok(files) = std::fs::read_dir(tenant_entry.path()) else { continue };

            // Find latest version per workflow ID
            let mut latest: HashMap<String, (u32, String)> = HashMap::new();
            for file in files.flatten() {
                let name = file.file_name().to_string_lossy().to_string();
                if !name.ends_with(".yaml") && !name.ends_with(".yml") { continue; }
                // Parse: {id}.v{N}.yaml or {id}.yaml
                let (wf_id, ver) = if let Some(pos) = name.rfind(".v") {
                    let ver_str = &name[pos+2..name.rfind('.').unwrap_or(name.len())];
                    let id = &name[..pos];
                    (id.to_string(), ver_str.parse::<u32>().unwrap_or(1))
                } else {
                    let id = name.trim_end_matches(".yaml").trim_end_matches(".yml");
                    (id.to_string(), 1)
                };

                if let Some((existing_ver, _)) = latest.get(&wf_id) {
                    if ver > *existing_ver {
                        latest.insert(wf_id, (ver, file.path().to_string_lossy().to_string()));
                    }
                } else {
                    latest.insert(wf_id, (ver, file.path().to_string_lossy().to_string()));
                }
            }

            for (_, (_, path)) in latest {
                match std::fs::read_to_string(&path) {
                    Ok(yaml) => match self.upload(&tenant, &yaml) {
                        Ok(entry) => {
                            // Auto-activate
                            let _ = self.activate(&tenant, &entry.id, entry.revision);
                            loaded += 1;
                        }
                        Err(e) => errors.push(format!("{}: {}", path, e)),
                    },
                    Err(e) => errors.push(format!("{}: {}", path, e)),
                }
            }
        }

        (loaded, errors)
    }

    pub fn count(&self) -> usize {
        self.slots.read().unwrap().len()
    }

    /// Get a compiled VilwGraph for a specific workflow version.
    pub fn get_graph(&self, tenant: &str, workflow_id: &str, revision: u32) -> Option<Arc<crate::graph::VilwGraph>> {
        let key = ns_key(tenant, workflow_id);
        let slots = self.slots.read().unwrap();
        let slot = slots.get(&key)?;
        let (graph, _, _) = slot.versions.get(&revision)?;
        Some(graph.clone())
    }
}

pub fn ns_key(tenant: &str, id: &str) -> String {
    format!("{}/{}", tenant, id)
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
