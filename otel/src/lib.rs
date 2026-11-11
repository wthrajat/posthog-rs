//! Links your OpenTelemetry spans to PostHog persons and sessions.
//!
//! `posthog_rs::tower::PostHogLayer` reads the `X-PostHog-Distinct-Id` and
//! `X-PostHog-Session-Id` headers that posthog-js sends with `tracing_headers`,
//! and scopes them as a [`posthog_rs::RequestContext`] for the request. This
//! crate copies those IDs onto spans as the `posthogDistinctId` and
//! `sessionId` attributes, which PostHog uses to link a trace to the person
//! and their session replay.
//!
//! ```no_run
//! use opentelemetry_sdk::trace::SdkTracerProvider;
//! use posthog_rs_otel::PostHogContextSpanProcessor;
//!
//! # async fn example() {
//! let client = posthog_rs::client("phc_your_project_token").await;
//! let provider = SdkTracerProvider::builder()
//!     .with_span_processor(PostHogContextSpanProcessor::new(&client))
//!     // .with_batch_exporter(your_exporter)
//!     .build();
//! # }
//! ```
//!
//! With Tower or axum, add `PostHogLayer` outside your tracing layer, so the
//! request span starts inside the request context and gets the IDs too:
//!
//! ```ignore
//! let app = Router::new()
//!     .route("/checkout", get(checkout))
//!     .layer(TraceLayer::new_for_http())
//!     .layer(PostHogLayer::new());
//! ```
//!
//! # Limits
//!
//! - The IDs come from request headers. They are analytics context, not
//!   authentication. Set `posthogDistinctId` yourself once you know the
//!   authenticated user; your value wins.
//! - Spans in tasks started with `tokio::spawn` get no IDs, because the request
//!   context doesn't follow them. Scope a clone of the context in the task to
//!   carry it over.
//! - The processor only covers spans that start inside the request context.
//!   If your tracing middleware runs outside `PostHogLayer`, the request span
//!   starts first; add `RequestSpanLayer` (feature `tower`) directly inside
//!   `PostHogLayer` to give it the IDs too.
//! - The IDs go to every exporter on the provider, not only PostHog.

mod processor;
mod request_ids;
#[cfg(feature = "tower")]
mod request_span;

pub use processor::PostHogContextSpanProcessor;
pub use request_ids::{DISTINCT_ID_ATTRIBUTE, IDENTITY_ATTRIBUTES, SESSION_ID_ATTRIBUTE};
#[cfg(feature = "tower")]
pub use request_span::{RequestSpanLayer, RequestSpanService};
