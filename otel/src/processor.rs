use std::time::Duration;

use opentelemetry::trace::Span as _;
use opentelemetry::Context;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{Span, SpanData, SpanProcessor};

use crate::request_ids::RequestIds;

/// Adds the current request's PostHog distinct ID and session ID to every
/// span that starts inside a [`posthog_rs::RequestContext`].
///
/// Register it next to your exporting processor. It only adds attributes, so
/// your provider, sampler and exporter stay as they are. A span that starts
/// with `sessionId` keeps it, a span that starts with any of
/// [`IDENTITY_ATTRIBUTES`](crate::IDENTITY_ATTRIBUTES) gets no distinct ID,
/// and a disabled client adds nothing.
///
/// With `tracing-opentelemetry`, the OpenTelemetry span starts the first time
/// something needs it. That is usually the first time the `tracing` span is
/// entered, with the default context activation, or earlier if its
/// OpenTelemetry context is read, for example by a child span. With context
/// activation off and nothing reading it, it starts when the span closes.
#[derive(Debug)]
pub struct PostHogContextSpanProcessor {
    client_enabled: bool,
}

impl PostHogContextSpanProcessor {
    pub fn new(client: &posthog_rs::Client) -> Self {
        Self {
            client_enabled: !client.is_disabled(),
        }
    }
}

impl SpanProcessor for PostHogContextSpanProcessor {
    fn on_start(&self, span: &mut Span, _parent: &Context) {
        if !self.client_enabled || !span.is_recording() {
            return;
        }
        let Some(ids) = RequestIds::current() else {
            return;
        };
        let existing = span.exported_data();
        let span_has_key = |key: &str| {
            existing.as_ref().is_some_and(|data| {
                data.attributes
                    .iter()
                    .any(|attribute| attribute.key.as_str() == key)
            })
        };
        for attribute in ids.attributes_missing_from(span_has_key) {
            span.set_attribute(attribute);
        }
    }

    fn on_end(&self, _span: SpanData) {}

    fn force_flush(&self) -> OTelSdkResult {
        Ok(())
    }

    fn shutdown_with_timeout(&self, _timeout: Duration) -> OTelSdkResult {
        Ok(())
    }
}
