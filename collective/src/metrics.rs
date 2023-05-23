#[cfg(feature = "metrics")]
pub mod counters {
    use std::sync::atomic::AtomicU64;

    pub static MUTEX_CONTENDED: AtomicU64 = AtomicU64::new(0);
    pub static MUTEX_UNCONTENDED: AtomicU64 = AtomicU64::new(0);
}

#[cfg(feature = "metrics")]
pub mod timers {
    use std::sync::atomic::AtomicU64;

    pub static BARRIER: AtomicU64 = AtomicU64::new(0);
    pub static COMPUTE: AtomicU64 = AtomicU64::new(0);
    pub static COPY: AtomicU64 = AtomicU64::new(0);
    pub static MUTEX: AtomicU64 = AtomicU64::new(0);
    pub static TOTAL: AtomicU64 = AtomicU64::new(0);
    pub static ZERO: AtomicU64 = AtomicU64::new(0);
}

#[cfg(feature = "metrics")]
macro_rules! increment {
    ($counter:path) => {
        $counter.fetch_add(1, ::std::sync::atomic::Ordering::AcqRel);
    };
}

#[cfg(not(feature = "metrics"))]
macro_rules! increment {
    ($_:path) => {};
}

pub(crate) use increment;

#[cfg(feature = "metrics")]
macro_rules! time {
    ($timer:path, $block:block) => {{
        let start = ::std::time::Instant::now();
        let value = $block;
        let end = ::std::time::Instant::now();
        $timer.fetch_add(
            (end - start).as_nanos() as u64,
            ::std::sync::atomic::Ordering::AcqRel,
        );
        value
    }};
}

#[cfg(not(feature = "metrics"))]
macro_rules! time {
    ($_:path, $block:block) => {
        $block
    };
}

pub(crate) use time;

#[cfg(feature = "metrics")]
pub fn dump() {
    use std::sync::atomic::AtomicU64;
    use std::sync::atomic::Ordering;

    let total = timers::TOTAL.load(Ordering::Acquire);

    let contended = counters::MUTEX_CONTENDED.load(Ordering::Acquire);
    let uncontended = counters::MUTEX_UNCONTENDED.load(Ordering::Acquire);

    let precision = |value: u64| match value {
        0..=999 => 3,
        1_000..=9_999 => 2,
        10_000..=99_999 => 1,
        _ => 0,
    };

    let category = |name, timer: &AtomicU64| {
        let timer = timer.load(Ordering::Acquire);
        eprintln!(
            "\t{}: {:.*}us ({:.2}%)",
            name,
            precision(timer),
            timer as f64 / 1e3,
            timer as f64 * 1e2 / total as f64,
        );
    };

    eprintln!("total: {:.*}us", precision(total), total as f64 / 1e3);
    category("zero", &timers::ZERO);
    category("copy", &timers::COPY);
    category("compute", &timers::COMPUTE);
    category("barrier", &timers::BARRIER);
    category("mutex", &timers::MUTEX);
    eprintln!(
        "\tmutex-uncontended: {}/{} ({:.2}%)",
        uncontended,
        uncontended + contended,
        uncontended as f64 * 100.0 / ((contended + uncontended) as f64),
    );
}

#[cfg(not(feature = "metrics"))]
pub fn dump() {}

#[cfg(feature = "metrics")]
pub fn reset() {
    use std::sync::atomic::Ordering;

    counters::MUTEX_CONTENDED.store(0, Ordering::Release);
    counters::MUTEX_UNCONTENDED.store(0, Ordering::Release);

    timers::BARRIER.store(0, Ordering::Release);
    timers::COPY.store(0, Ordering::Release);
    timers::COMPUTE.store(0, Ordering::Release);
    timers::MUTEX.store(0, Ordering::Release);
    timers::TOTAL.store(0, Ordering::Release);
    timers::ZERO.store(0, Ordering::Release);
}

#[cfg(not(feature = "metrics"))]
pub fn reset() {}
