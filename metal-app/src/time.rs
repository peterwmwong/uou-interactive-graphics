use std::time::Instant;

// TODO: Consider using a macro to limit the performance impact.
// - Even with inlining, can cause wildly different/bad code generation.
// - Reducing the number of debug_time's in Model, yielded a dramatic code reduction.
#[inline]
pub fn debug_time<T>(label: &'static str, f: impl FnOnce() -> T) -> T {
    #[cfg(debug_assertions)]
    {
        const MICROS_PER_MILLI: u128 = 1000;
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
