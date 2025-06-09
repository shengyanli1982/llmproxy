use std::sync::atomic::{AtomicU64, Ordering};

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 生成下一个ID
pub fn next_id() -> u64 {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
