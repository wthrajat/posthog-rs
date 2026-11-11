use opentelemetry::KeyValue;
use posthog_rs::RequestContext;

/// Span attribute that PostHog reads as the person's distinct ID.
pub const DISTINCT_ID_ATTRIBUTE: &str = "posthogDistinctId";

/// Span attribute that PostHog reads as the session ID.
pub const SESSION_ID_ATTRIBUTE: &str = "sessionId";

/// Span attributes that already identify the person. A span with any of them
/// gets no `posthogDistinctId` from the request, so the application's
/// identity wins over header-derived identity.
pub const IDENTITY_ATTRIBUTES: [&str; 4] = [
    DISTINCT_ID_ATTRIBUTE,
    "posthog.distinct_id",
    "user.id",
    "enduser.id",
];

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

    /// Attributes for the IDs the span doesn't already have, so identity the
    /// application set itself always wins.
    pub(crate) fn attributes_missing_from(
        self,
        span_has_key: impl Fn(&str) -> bool,
    ) -> impl Iterator<Item = KeyValue> {
        let has_identity = IDENTITY_ATTRIBUTES.iter().any(|key| span_has_key(key));
        let has_session = span_has_key(SESSION_ID_ATTRIBUTE);

        let distinct_id = self
            .distinct_id
            .filter(|_| !has_identity)
            .map(|id| KeyValue::new(DISTINCT_ID_ATTRIBUTE, id));
        let session_id = self
            .session_id
            .filter(|_| !has_session)
            .map(|id| KeyValue::new(SESSION_ID_ATTRIBUTE, id));
        distinct_id.into_iter().chain(session_id)
    }
}

fn non_empty(id: Option<&str>) -> Option<String> {
    id.filter(|id| !id.is_empty()).map(str::to_owned)
}
