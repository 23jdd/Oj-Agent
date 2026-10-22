//! A stopped worker can finish later, but it no longer owns the UI's busy state.
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
    active: Option<Job>,
}
impl Jobs {
    pub fn begin(&mut self) -> Job {
        self.stop();
        let job = Job {
            id: Uuid::new_v4(),
            cancel: Arc::default(),
        };
        self.active = Some(job.clone());
        job
    }
    pub fn is_current(&self, id: Uuid) -> bool {
        self.active.as_ref().is_some_and(|job| job.id == id)
    }
    pub fn finish(&mut self, id: Uuid) -> bool {
        if !self.is_current(id) {
            return false;
        }
        self.active = None;
        true
    }
    pub fn stop(&mut self) {
        if let Some(job) = self.active.take() {
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
        let stopped = jobs.begin();
        jobs.stop();
        assert!(stopped.cancel.load(Ordering::Relaxed));
        assert!(!jobs.is_current(stopped.id));
        let next = jobs.begin();
        assert!(!next.cancel.load(Ordering::Relaxed));
        assert!(!jobs.finish(stopped.id));
        assert!(jobs.is_current(next.id));
        assert!(jobs.finish(next.id));
        assert!(!jobs.is_current(next.id));
        assert!(!jobs.finish(next.id));
    }
}
