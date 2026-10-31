use opentelemetry::KeyValue;
use posthog_rs::RequestContext;

/// Span attribute that PostHog reads as the person's distinct ID.
pub const DISTINCT_ID_ATTRIBUTE: &str = "posthogDistinctId";

/// Span attribute that PostHog reads as the session ID.
pub const SESSION_ID_ATTRIBUTE: &str = "sessionId";

/// The PostHog IDs of the request being handled.
pub(crate) struct RequestIds {
    distinct_id: Option<String>,
    session_id: Option<String>,
}

impl RequestIds {
    /// Reads the IDs from the active `RequestContext`. Returns `None` outside a
    /// request or when the request carries neither ID.
    pub(crate) fn current() -> Option<Self> {
        let context = RequestContext::current()?;
        let ids = Self {
            distinct_id: non_empty(context.get_distinct_id()),
            session_id: non_empty(context.get_session_id()),
        };
        let has_any_id = ids.distinct_id.is_some() || ids.session_id.is_some();
        has_any_id.then_some(ids)
    }

    /// Attributes for the IDs whose keys the span doesn't already have, so
    /// identity the application set itself always wins.
    pub(crate) fn attributes_missing_from(
        self,
        span_has_key: impl Fn(&str) -> bool,
    ) -> Vec<KeyValue> {
        let candidates = [
            (DISTINCT_ID_ATTRIBUTE, self.distinct_id),
            (SESSION_ID_ATTRIBUTE, self.session_id),
        ];
        candidates
            .into_iter()
            .filter(|(key, _)| !span_has_key(key))
            .filter_map(|(key, id)| id.map(|id| KeyValue::new(key, id)))
            .collect()
    }
}

fn non_empty(id: Option<&str>) -> Option<String> {
    id.filter(|id| !id.is_empty()).map(str::to_owned)
}
