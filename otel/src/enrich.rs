use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::request_ids::RequestIds;

/// Adds the current request's PostHog IDs to the current `tracing` span.
///
/// [`PostHogContextSpanProcessor`](crate::PostHogContextSpanProcessor) only
/// sees spans that start inside the request context. Call this where the
/// context exists to cover a span that started earlier, such as an HTTP
/// request span created by middleware outside `PostHogLayer`.
///
/// A field the span declares with the same name as an ID attribute is treated
/// as the application's own identity and left alone. A disabled client adds
/// nothing.
pub fn enrich_current_span(client: &posthog_rs::Client) {
    if client.is_disabled() {
        return;
    }
    let span = tracing::Span::current();
    if span.is_disabled() {
        return;
    }
    let Some(ids) = RequestIds::current() else {
        return;
    };
    for attribute in ids.attributes_missing_from(|key| span.has_field(key)) {
        span.set_attribute(attribute.key, attribute.value);
    }
}
