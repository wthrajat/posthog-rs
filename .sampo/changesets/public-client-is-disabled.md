---
cargo/posthog-rs: minor
---

Make `Client::is_disabled()` public on the async and blocking clients, and available without the `error-tracking` feature, so integrations can stay inert for a disabled client.
