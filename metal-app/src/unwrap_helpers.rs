#[inline(always)]
pub fn unwrap_result_dcheck<T, E>(r: Result<T, E>, msg: &'static str) -> T {
    debug_assert!(r.is_ok(), "{msg}");
    unsafe { r.unwrap_unchecked() }
}

#[inline(always)]
pub fn unwrap_option_dcheck<T>(r: Option<T>, msg: &'static str) -> T {
    debug_assert!(r.is_some(), "{msg}");
    unsafe { r.unwrap_unchecked() }
}
