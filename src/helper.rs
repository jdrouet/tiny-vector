#[cfg(feature = "source-sysinfo")]
#[inline(always)]
pub fn default_true() -> bool {
    true
}

pub fn now() -> u64 {
    let delta = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    delta.as_secs()
}
