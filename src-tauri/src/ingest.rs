use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};

use lumio_core::ingest::IngestPlan;

#[derive(Default)]
pub struct IngestRuntime {
    plans: Mutex<HashMap<String, IngestPlan>>,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    running: Mutex<HashSet<String>>,
}

impl IngestRuntime {
    fn plans(&self) -> MutexGuard<'_, HashMap<String, IngestPlan>> {
        self.plans
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn cancels(&self) -> MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.cancels
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn running(&self) -> MutexGuard<'_, HashSet<String>> {
        self.running
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Store a fresh plan. Keeping a small window prevents abandoned dry runs
    /// from accumulating in memory during a long session.
    pub fn store(&self, plan: IngestPlan) {
        let mut plans = self.plans();
        plans.insert(plan.id.clone(), plan);
        if plans.len() > 12 {
            let mut ids: Vec<String> = plans.keys().cloned().collect();
            ids.sort();
            for id in ids.into_iter().take(plans.len() - 12) {
                plans.remove(&id);
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<IngestPlan> {
        self.plans().get(id).cloned()
    }

    pub fn update(&self, plan: IngestPlan) {
        self.plans().insert(plan.id.clone(), plan);
    }

    pub fn begin(&self, id: &str) -> Option<IngestPlan> {
        let Some(plan) = self.plans().remove(id) else {
            return None;
        };
        if !self.running().insert(id.to_string()) {
            // Another run won the race; put the plan back untouched.
            self.plans().insert(id.to_string(), plan);
            return None;
        }
        self.cancels()
            .insert(id.to_string(), Arc::new(AtomicBool::new(false)));
        Some(plan)
    }

    pub fn finish(&self, id: &str) {
        self.running().remove(id);
        self.cancels().remove(id);
    }

    pub fn cancel_token(&self, id: &str) -> Option<Arc<AtomicBool>> {
        self.cancels().get(id).cloned()
    }

    pub fn cancel(&self, id: &str) -> bool {
        let token = self.cancel_token(id);
        if let Some(token) = token {
            token.store(true, std::sync::atomic::Ordering::SeqCst);
            true
        } else {
            false
        }
    }
}
