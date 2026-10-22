//! Each conversation owns its worker and cancellation token independently.
use std::collections::HashMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use uuid::Uuid;

#[derive(Clone)]
pub struct Job {
    pub id: Uuid,
    pub cancel: Arc<AtomicBool>,
}
#[derive(Default)]
pub struct Jobs {
    active: HashMap<String, Job>,
}
impl Jobs {
    pub fn begin(&mut self, session: &str) -> Option<Job> {
        if self.busy(session) {
            return None;
        }
        let job = Job {
            id: Uuid::new_v4(),
            cancel: Arc::default(),
        };
        self.active.insert(session.into(), job.clone());
        Some(job)
    }
    pub fn busy(&self, session: &str) -> bool {
        self.active.contains_key(session)
    }
    pub fn owner(&self, id: Uuid) -> Option<&str> {
        self.active
            .iter()
            .find(|(_, job)| job.id == id)
            .map(|(session, _)| session.as_str())
    }
    pub fn finish(&mut self, id: Uuid) -> Option<String> {
        let session = self.owner(id)?.to_owned();
        self.active.remove(&session);
        Some(session)
    }
    pub fn stop(&mut self, session: &str) -> bool {
        if let Some(job) = self.active.remove(session) {
            job.cancel.store(true, Ordering::Relaxed);
            return true;
        }
        false
    }
    pub fn stop_all(&mut self) {
        for (_, job) in self.active.drain() {
            job.cancel.store(true, Ordering::Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stopped_worker_cannot_relock_or_finish_the_next_conversations_job() {
        let mut jobs = Jobs::default();
        let stopped = jobs.begin("first").unwrap();
        jobs.stop("first");
        assert!(stopped.cancel.load(Ordering::Relaxed));
        assert!(jobs.owner(stopped.id).is_none());
        let next = jobs.begin("first").unwrap();
        assert!(!next.cancel.load(Ordering::Relaxed));
        assert!(jobs.finish(stopped.id).is_none());
        assert_eq!(jobs.owner(next.id), Some("first"));
        assert_eq!(jobs.finish(next.id).as_deref(), Some("first"));
        assert!(jobs.owner(next.id).is_none());
        assert!(jobs.finish(next.id).is_none());
    }
    #[test]
    fn concurrent_conversations_can_finish_and_stop_independently() {
        let mut jobs = Jobs::default();
        let first = jobs.begin("first").unwrap();
        let second = jobs.begin("second").unwrap();
        assert!(jobs.begin("first").is_none());
        assert_eq!(jobs.owner(first.id), Some("first"));
        assert!(jobs.stop("first"));
        assert!(!second.cancel.load(Ordering::Relaxed));
        assert!(jobs.busy("second"));
        assert!(jobs.finish(first.id).is_none());
        assert_eq!(jobs.finish(second.id).as_deref(), Some("second"));
        let third = jobs.begin("third").unwrap();
        jobs.stop_all();
        assert!(first.cancel.load(Ordering::Relaxed));
        assert!(third.cancel.load(Ordering::Relaxed));
    }
}
