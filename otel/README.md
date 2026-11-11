# posthog-rs-otel

Links your OpenTelemetry spans to PostHog persons and session replays.

posthog-js sends `X-PostHog-Distinct-Id` and `X-PostHog-Session-Id` with
requests to the hosts in its `tracing_headers` option. `PostHogLayer` from
`posthog-rs` scopes those IDs as a `RequestContext` for the request. This crate
copies them onto your spans as the `posthogDistinctId` and `sessionId`
attributes. PostHog Tracing uses those attributes to show the person and a
link to their recording on each trace.

It only adds attributes. Your tracer provider, sampler and exporter stay as
they are, and OpenTelemetry stays out of the `posthog-rs` crate.

## Setup

Add the processor to your tracer provider:

```rust
use opentelemetry_sdk::trace::SdkTracerProvider;
use posthog_rs_otel::PostHogContextSpanProcessor;

let client = posthog_rs::client("phc_your_project_token").await;
let provider = SdkTracerProvider::builder()
    .with_span_processor(PostHogContextSpanProcessor::new(&client))
    .with_batch_exporter(exporter)
    .build();
```

With axum or Tower, add `PostHogLayer` outside your tracing layer. The request
span then starts inside the request context and gets the IDs, along with every
span below it:

```rust
let app = Router::new()
    .route("/checkout", get(checkout))
    .layer(TraceLayer::new_for_http())
    .layer(PostHogLayer::new());
```

If your tracing middleware has to run outside `PostHogLayer`, the request span
starts before the IDs are known. Enable the `tower` feature and add
`RequestSpanLayer` directly inside `PostHogLayer`. It adds the IDs to that
request span:

```rust
use posthog_rs_otel::RequestSpanLayer;

let app = Router::new()
    .route("/checkout", get(checkout))
    .layer(RequestSpanLayer::new(&client))
    .layer(PostHogLayer::new())
    .layer(TraceLayer::new_for_http());
```

Use one order or the other. With `PostHogLayer` outside, the processor already
covers the request span, and `RequestSpanLayer` would add the IDs a second
time.

To send spans to PostHog, export them over OTLP/HTTP to
`<host>/i/v1/traces` with `Authorization: Bearer <project token>`.

## Behavior

- A span that already has `sessionId` keeps it.
- A span that already has any of `posthogDistinctId`, `posthog.distinct_id`,
  `user.id` or `enduser.id` gets no `posthogDistinctId`, so your identity wins
  over the header's (`IDENTITY_ATTRIBUTES`).
- At the span's attribute limit, the IDs are dropped. Nothing is evicted.
- An ID the request doesn't have is left off. Nothing is generated.
- A disabled client adds nothing.
- Spans that aren't recording are skipped.

## Limits

- The IDs come from request headers. They're analytics context, not
  authentication. Once you know the signed-in user, scope a context with
  `.distinct_id(authenticated_id)`, or set `posthogDistinctId` on the span.
- Spans in tasks started with `tokio::spawn` get no IDs, because the request
  context doesn't follow them. Scope a clone of the context in the task.
- The processor only covers spans that start inside the request context. A
  span that started earlier gets the IDs only through `RequestSpanLayer`, and
  only if it's the current span when that layer runs.
- `RequestSpanLayer` checks the request span's fields, not attributes set with
  `OpenTelemetrySpanExt::set_attribute`: Rust's OpenTelemetry API can't read
  those back. Set your own IDs as span fields, or after the layer runs.
- With `tracing-opentelemetry`, the OpenTelemetry span starts the first time
  something needs it: usually when the `tracing` span is first entered, or
  earlier if its OpenTelemetry context is read, for example by a child span.
  With context activation off and nothing reading it, it starts when the span
  closes. Make sure that moment is inside the request context.
- The attributes go to every exporter on the provider, not only PostHog.
- Needs the `async-client` feature of `posthog-rs`, which the crate enables.
  `RequestSpanLayer` needs this crate's `tower` feature, which adds `tracing`
  and `tracing-opentelemetry`. Without it, the crate depends only on the
  OpenTelemetry API and SDK.

## Example

[`examples/axum_checkout.rs`](examples/axum_checkout.rs) serves a page with
posthog-js and a traced axum backend:

```bash
export POSTHOG_API_TOKEN=phc_your_project_token
cargo run -p posthog-rs-otel --example axum_checkout
```

Open http://localhost:3000 and click "Check out". Then press Ctrl+C to flush,
and open Tracing in PostHog. Each `axum-checkout` trace shows the person and
links to the session replay.
