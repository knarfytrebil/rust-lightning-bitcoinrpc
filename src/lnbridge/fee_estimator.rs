use lightning::chain::chaininterface;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::cmp;

pub struct FeeEstimator {
    // est
    background_est: AtomicUsize,
    normal_est: AtomicUsize,
    high_prio_est: AtomicUsize,
}

impl FeeEstimator {
    pub fn new() -> Self {
        Self {
            background_est: AtomicUsize::new(0),
            normal_est: AtomicUsize::new(0),
            high_prio_est: AtomicUsize::new(0),
        }
    }
}

impl chaininterface::FeeEstimator for FeeEstimator {
    fn get_est_sat_per_1000_weight(&self, conf_target: chaininterface::ConfirmationTarget) -> u64 {
        cmp::max(match conf_target {
            chaininterface::ConfirmationTarget::Background => self.background_est.load(Ordering::Acquire) as u64,
            chaininterface::ConfirmationTarget::Normal => self.normal_est.load(Ordering::Acquire) as u64,
            chaininterface::ConfirmationTarget::HighPriority => self.high_prio_est.load(Ordering::Acquire) as u64,
        }, 253)
    }
}
