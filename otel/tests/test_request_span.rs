mod support;

use axum::body::Body;
use axum::http::Request;
use axum::routing::get;
use axum::Router;
use posthog_rs::tower::PostHogLayer;
use posthog_rs_otel::RequestSpanLayer;
use support::{disabled_client, enabled_client, values, Recorder};
use tower::ServiceExt;
use tower_http::trace::TraceLayer;
use tracing::Instrument;

#[tokio::test]
async fn tags_a_request_span_that_started_before_posthog_layer() {
    let client = enabled_client().await;
    let recorder = Recorder::new(&client);
    let _tracing = recorder.install_tracing();

    let app = Router::new()
        .route("/checkout", get(checkout))
        .layer(RequestSpanLayer::new(&client))
        .layer(PostHogLayer::new())
        .layer(TraceLayer::new_for_http());
    send_checkout(app).await;

    for name in ["request", "db.query"] {
        let span = recorder.span(name);
        assert_eq!(values(&span, "posthogDistinctId"), ["user-1"], "{name}");
        assert_eq!(values(&span, "sessionId"), ["session-1"], "{name}");
    }
}

#[tokio::test]
async fn keeps_identity_the_request_span_was_created_with() {
    let client = enabled_client().await;
    let recorder = Recorder::new(&client);
    let _tracing = recorder.install_tracing();

    let trace_layer = TraceLayer::new_for_http().make_span_with(|_: &Request<Body>| {
        tracing::info_span!("request", user.id = "app-user", sessionId = "app-session")
    });
    let app = Router::new()
        .route("/checkout", get(checkout))
        .layer(RequestSpanLayer::new(&client))
        .layer(PostHogLayer::new())
        .layer(trace_layer);
    send_checkout(app).await;

    let span = recorder.span("request");
    assert!(values(&span, "posthogDistinctId").is_empty());
    assert_eq!(values(&span, "user.id"), ["app-user"]);
    assert_eq!(values(&span, "sessionId"), ["app-session"]);
}

#[tokio::test]
async fn adds_nothing_for_a_disabled_client() {
    let client = disabled_client().await;
    let recorder = Recorder::new(&client);
    let _tracing = recorder.install_tracing();

    let app = Router::new()
        .route("/checkout", get(checkout))
        .layer(RequestSpanLayer::new(&client))
        .layer(PostHogLayer::new())
        .layer(TraceLayer::new_for_http());
    send_checkout(app).await;

    let span = recorder.span("request");
    assert!(values(&span, "posthogDistinctId").is_empty());
    assert!(values(&span, "sessionId").is_empty());
}

#[tokio::test]
async fn leaves_a_request_without_ids_untouched() {
    let client = enabled_client().await;
    let recorder = Recorder::new(&client);
    let _tracing = recorder.install_tracing();

    let app = Router::new()
        .route("/checkout", get(checkout))
        .layer(RequestSpanLayer::new(&client))
        .layer(PostHogLayer::new())
        .layer(TraceLayer::new_for_http());
    let anonymous = Request::get("/checkout")
        .body(Body::empty())
        .expect("request");
    send(app, anonymous).await;

    let span = recorder.span("request");
    assert!(values(&span, "posthogDistinctId").is_empty());
    assert!(values(&span, "sessionId").is_empty());
}

#[tokio::test]
async fn passes_requests_through_without_a_tracing_subscriber() {
    let client = enabled_client().await;

    let app = Router::new()
        .route("/checkout", get(checkout))
        .layer(RequestSpanLayer::new(&client))
        .layer(PostHogLayer::new())
        .layer(TraceLayer::new_for_http());

    assert_eq!(send_checkout(app).await, "ok");
}

async fn send_checkout(app: Router) -> String {
    let checkout_request = Request::get("/checkout")
        .header("X-PostHog-Distinct-Id", "user-1")
        .header("X-PostHog-Session-Id", "session-1")
        .body(Body::empty())
        .expect("request");
    send(app, checkout_request).await
}

async fn send(app: Router, request: Request<Body>) -> String {
    let response = app.oneshot(request).await.expect("response");
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    String::from_utf8(body.to_vec()).expect("utf-8 body")
}

async fn checkout() -> &'static str {
    async { tokio::task::yield_now().await }
        .instrument(tracing::info_span!("db.query"))
        .await;
    "ok"
}
