# Global ARAM Crawler Adaptive Rate Limiting Walkthrough

## Summary of Changes
To ensure the Global ARAM Crawler match collector automatically converges on the optimal collective request throughput across all worker threads without getting rate-limited (HTTP 429) or timed out, we introduced a shared adaptive rate limiter implementing **AIMD** (Additive Increase / Multiplicative Decrease).

### 1. Cross-Worker `AdaptiveRateLimiter`
- **Location**: [`src-tauri/src/agent/mod.rs`](src-tauri/src/agent/mod.rs#L2245-L2320)
- **Shared Coordination**: Instead of isolated per-worker pacers that could burst concurrently, an `Arc<tokio::sync::Mutex<AdaptiveRateLimiter>>` is instantiated once in `collect_global_aram` and shared across all active workers.
- **Dynamic Capacity**:
  - Initial safe rate: 60 RPM (1 request/second aggregate across all workers).
  - Bounds: Clamped between 20 RPM and 150 RPM.
- **AIMD Convergence**:
  - **Additive Increase (`record_success`)**: On every 8 sustained successful responses, aggregate capacity increments by `+2 RPM` up to the max safe limit.
  - **Multiplicative Decrease (`record_backoff`)**:
    - On HTTP 429: Cuts rate by 50% to immediately relieve LCU pressure.
    - On transient errors / timeouts: Cuts rate by 25%.
- **Anti-Burst Pacing (`pace_next`)**: Enforces inter-request intervals (`60,000 / current_rpm` ms) across whichever worker executes next.

### 2. Integration with Worker Pipeline & Circuit Breaker
- Both match history queries (`/lol-match-history/v1/products/lol/.../matches`) and game detail queries (`/lol-match-history/v1/games/{id}`) now invoke `rate_limiter.lock().await.pace_next().await`.
- Successful calls trigger `record_success()`, logging rate changes when throughput increases.
- 429 or timeout errors notify `record_backoff()`, adjusting collective throughput downward before retrying or triggering circuit breaker cooldowns.

### 3. Unit Verification
- Added `test_adaptive_rate_limiter_aimd` in `src-tauri/src/agent/mod.rs` verifying threshold-based additive increase and multiplicative step-downs under error conditions.

