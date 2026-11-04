---
cargo/posthog-rs-otel: minor
---

Add `posthog-rs-otel`, an OpenTelemetry integration that copies the active `RequestContext`'s distinct ID and session ID onto spans as `posthogDistinctId` and `sessionId`. Register `PostHogContextSpanProcessor` on your tracer provider, or call `enrich_current_span` for a span that started before the request context. Existing identity attributes are preserved, and disabled clients add nothing.
