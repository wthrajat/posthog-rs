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
/// your provider, sampler and exporter stay as they are. Spans that already
/// have `posthogDistinctId` or `sessionId` keep their own value, and a
/// disabled client adds nothing.
///
/// With `tracing-opentelemetry`, a span starts when it is first entered, so
/// any span entered inside the request context is covered.
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
        let existing_keys = attribute_keys(span);
        let missing = ids.attributes_missing_from(|key| existing_keys.iter().any(|k| k == key));
        for attribute in missing {
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

/// The SDK only exposes a span's attributes through a copy of its data. At
/// start that copy is small: just the attributes the span was created with.
fn attribute_keys(span: &Span) -> Vec<String> {
    let Some(data) = span.exported_data() else {
        return Vec::new();
    };
    data.attributes
        .into_iter()
        .map(|attribute| attribute.key.to_string())
        .collect()
}
