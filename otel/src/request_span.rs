use std::task::{Context, Poll};

use tower_layer::Layer;
use tower_service::Service;
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::request_ids::RequestIds;

/// Adds the request's PostHog IDs to a request span that started before
/// `PostHogLayer` read them.
///
/// You only need it when your tracing middleware runs outside `PostHogLayer`.
/// Add it directly inside `PostHogLayer`, so the current span is the request
/// span:
///
/// ```ignore
/// let app = Router::new()
///     .route("/checkout", get(checkout))
///     .layer(RequestSpanLayer::new(&client))
///     .layer(PostHogLayer::new())
///     .layer(TraceLayer::new_for_http());
/// ```
///
/// Use one order or the other: with `PostHogLayer` outside your tracing layer,
/// the processor already covers the request span, and this layer would add
/// the IDs a second time.
///
/// The span keeps any ID it was created with: a `sessionId` field blocks the
/// session ID, and any of [`IDENTITY_ATTRIBUTES`](crate::IDENTITY_ATTRIBUTES)
/// blocks the distinct ID. Attributes set with
/// [`OpenTelemetrySpanExt::set_attribute`] can't be read back, so set your
/// own IDs as span fields, or after this layer runs.
#[derive(Clone, Debug)]
pub struct RequestSpanLayer {
    client_enabled: bool,
}

impl RequestSpanLayer {
    pub fn new(client: &posthog_rs::Client) -> Self {
        Self {
            client_enabled: !client.is_disabled(),
        }
    }
}

impl<S> Layer<S> for RequestSpanLayer {
    type Service = RequestSpanService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestSpanService {
            inner,
            client_enabled: self.client_enabled,
        }
    }
}

/// The service that [`RequestSpanLayer`] wraps around yours.
#[derive(Clone, Debug)]
pub struct RequestSpanService<S> {
    inner: S,
    client_enabled: bool,
}

impl<S, Request> Service<Request> for RequestSpanService<S>
where
    S: Service<Request>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        if self.client_enabled {
            add_ids_to_current_span();
        }
        self.inner.call(request)
    }
}

// `set_attribute` doesn't start a span that hasn't started yet; the processor
// then sees these attributes at start and adds nothing twice. A span that
// isn't recording ignores them.
fn add_ids_to_current_span() {
    let span = tracing::Span::current();
    if span.is_none() {
        return;
    }
    let Some(ids) = RequestIds::current() else {
        return;
    };
    for attribute in ids.attributes_missing_from(|key| span.has_field(key)) {
        span.set_attribute(attribute.key, attribute.value);
    }
}
