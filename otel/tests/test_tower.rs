mod support;

use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::Router;
use posthog_rs::tower::PostHogLayer;
use support::{enabled_client, values, Recorder};
use tower::ServiceExt;
use tower_http::trace::TraceLayer;
use tracing::Instrument;

#[tokio::test]
async fn posthog_layer_outside_the_trace_layer_covers_every_span() {
    let recorder = Recorder::new(&enabled_client().await);
    let _tracing = recorder.install_tracing();

    let app = Router::new()
        .route("/checkout", get(checkout))
        .layer(TraceLayer::new_for_http())
        .layer(PostHogLayer::new());
    let checkout_request = Request::get("/checkout")
        .header("X-PostHog-Distinct-Id", "user-1")
        .header("X-PostHog-Session-Id", "session-1")
        .body(Body::empty())
        .expect("request");

    let response = app.oneshot(checkout_request).await.expect("response");
    axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");

    for name in ["request", "db.query"] {
        let span = recorder.span(name);
        assert_eq!(values(&span, "posthogDistinctId"), ["user-1"], "{name}");
        assert_eq!(values(&span, "sessionId"), ["session-1"], "{name}");
    }
}

async fn checkout() -> &'static str {
    async { tokio::task::yield_now().await }
        .instrument(tracing::info_span!("db.query"))
        .await;
    "ok"
}
