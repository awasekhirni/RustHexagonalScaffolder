//! Rate limiter — token bucket via `governor` (stub: just constructs a
//! permit-less limiter so the code compiles without the dependency).

/// Builds the shared rate-limiter state.
///
/// Stub: production code uses `governor::RateLimiter::direct(...)` and
/// returns a stateful limiter; this returns `()` so handlers can compile.
#[must_use]
pub fn build_limiter(_per_second: u32, _burst: u32) -> () {
    // no-op stub
}
