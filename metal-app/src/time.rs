use std::time::Instant;

const MICROS_PER_MILLI: u128 = 1000;

pub fn debug_time<T>(label: &'static str, f: impl FnOnce() -> T) -> T {
    #[cfg(debug_assertions)]
    {
        let now = Instant::now();
        let r = f();
        let elapsed = now.elapsed();
        let elapsed_micro = elapsed.as_micros();
        let (elapsed_display, unit) = if elapsed_micro > MICROS_PER_MILLI {
            (elapsed_micro / MICROS_PER_MILLI, "ms")
        } else {
            (elapsed_micro, "μ")
        };
        println!("[{label:<40}] {:>6} {}", elapsed_display, unit);
        return r;
    }
    #[cfg(not(debug_assertions))]
    {
        return f();
    }
}
