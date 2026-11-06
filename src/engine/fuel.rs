use std::sync::atomic::{AtomicU64, Ordering};

/// FuelMeter tracks discrete instruction execution steps.
/// When fuel reaches zero, execution traps in sub-microseconds without waiting for wall-clock timeouts.
#[derive(Debug)]
pub struct FuelMeter {
    remaining: AtomicU64,
    initial_budget: u64,
}

impl FuelMeter {
    pub fn new(budget: u64) -> Self {
        Self {
            remaining: AtomicU64::new(budget),
            initial_budget: budget,
        }
    }

    #[inline(always)]
    pub fn consume(&self, amount: u64) -> Result<u64, ()> {
        let mut current = self.remaining.load(Ordering::Relaxed);
        loop {
            if current < amount {
                self.remaining.store(0, Ordering::Relaxed);
                return Err(()); // Out of fuel trap
            }
            let next = current - amount;
            match self.remaining.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(next),
                Err(actual) => current = actual,
            }
        }
    }

    pub fn consumed(&self) -> u64 {
        self.initial_budget.saturating_sub(self.remaining.load(Ordering::Relaxed))
    }

    pub fn remaining(&self) -> u64 {
        self.remaining.load(Ordering::Relaxed)
    }

    pub fn reset(&self, new_budget: u64) {
        self.remaining.store(new_budget, Ordering::SeqCst);
    }
}
