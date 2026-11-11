---
cargo/posthog-rs-otel: minor
---

Add `posthog-rs-otel`, an OpenTelemetry integration that copies the active `RequestContext`'s distinct ID and session ID onto spans as `posthogDistinctId` and `sessionId`. Register `PostHogContextSpanProcessor` on your tracer provider for spans that start inside the request context. If your tracing middleware runs outside `PostHogLayer`, enable the `tower` feature and add `RequestSpanLayer` directly inside `PostHogLayer` to tag the request span too. A span keeps its own `sessionId`, and any of `posthogDistinctId`, `posthog.distinct_id`, `user.id` or `enduser.id` keeps the distinct ID off. Disabled clients add nothing.
