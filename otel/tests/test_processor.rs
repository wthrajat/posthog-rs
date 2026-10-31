mod support;

use opentelemetry::trace::{Span as _, Tracer as _};
use opentelemetry::KeyValue;
use posthog_rs::RequestContext;
use support::{disabled_client, enabled_client, request, values, Recorder};
use tracing::Instrument;

const DISTINCT_ID: &str = "posthogDistinctId";
const SESSION_ID: &str = "sessionId";

#[tokio::test]
async fn stamps_spans_started_inside_the_request_context() {
    let recorder = Recorder::new(&enabled_client().await);
    let tracer = recorder.tracer();

    request("user-1", "session-1")
        .scope(async { tracer.in_span("db.query", |_| {}) })
        .await;

    let span = recorder.span("db.query");
    assert_eq!(values(&span, DISTINCT_ID), ["user-1"]);
    assert_eq!(values(&span, SESSION_ID), ["session-1"]);
}

#[tokio::test]
async fn leaves_spans_outside_a_request_untouched() {
    let recorder = Recorder::new(&enabled_client().await);

    recorder.tracer().in_span("startup", |_| {});

    let span = recorder.span("startup");
    assert!(values(&span, DISTINCT_ID).is_empty());
    assert!(values(&span, SESSION_ID).is_empty());
}

#[tokio::test]
async fn keeps_identity_the_span_was_created_with() {
    let recorder = Recorder::new(&enabled_client().await);
    let tracer = recorder.tracer();

    request("header-user", "session-1")
        .scope(async {
            let mut span = tracer
                .span_builder("authenticated")
                .with_attributes([KeyValue::new(DISTINCT_ID, "app-user")])
                .start(&tracer);
            span.end();
        })
        .await;

    let span = recorder.span("authenticated");
    assert_eq!(values(&span, DISTINCT_ID), ["app-user"]);
    assert_eq!(values(&span, SESSION_ID), ["session-1"]);
}

#[tokio::test]
async fn omits_ids_the_request_does_not_have() {
    let recorder = Recorder::new(&enabled_client().await);
    let tracer = recorder.tracer();

    RequestContext::new()
        .session_id("session-only")
        .scope(async { tracer.in_span("anonymous", |_| {}) })
        .await;

    let span = recorder.span("anonymous");
    assert!(values(&span, DISTINCT_ID).is_empty());
    assert_eq!(values(&span, SESSION_ID), ["session-only"]);
}

#[tokio::test]
async fn adds_nothing_for_a_disabled_client() {
    let recorder = Recorder::new(&disabled_client().await);
    let tracer = recorder.tracer();

    request("user-1", "session-1")
        .scope(async { tracer.in_span("db.query", |_| {}) })
        .await;

    let span = recorder.span("db.query");
    assert!(values(&span, DISTINCT_ID).is_empty());
    assert!(values(&span, SESSION_ID).is_empty());
}

#[tokio::test]
async fn keeps_concurrent_requests_apart() {
    let recorder = Recorder::new(&enabled_client().await);
    let tracer = recorder.tracer();

    let first = request("user-a", "session-a").scope(async {
        tokio::task::yield_now().await;
        tracer.in_span("request-a", |_| {});
    });
    let second = request("user-b", "session-b").scope(async {
        tokio::task::yield_now().await;
        tracer.in_span("request-b", |_| {});
    });
    tokio::join!(first, second);

    assert_eq!(values(&recorder.span("request-a"), DISTINCT_ID), ["user-a"]);
    assert_eq!(values(&recorder.span("request-b"), DISTINCT_ID), ["user-b"]);
}

#[tokio::test]
async fn does_not_follow_spawned_tasks() {
    let recorder = Recorder::new(&enabled_client().await);
    let tracer = recorder.tracer();

    request("user-1", "session-1")
        .scope(async {
            let spawned = tokio::spawn(async move { tracer.in_span("spawned", |_| {}) });
            spawned.await.expect("spawned task");
        })
        .await;

    assert!(values(&recorder.span("spawned"), DISTINCT_ID).is_empty());
}

#[tokio::test]
async fn stamps_tracing_spans_entered_inside_the_request_context() {
    let recorder = Recorder::new(&enabled_client().await);
    let _tracing = recorder.install_tracing();

    request("user-1", "session-1")
        .scope(async { tokio::task::yield_now().await }.instrument(tracing::info_span!("handler")))
        .await;

    let span = recorder.span("handler");
    assert_eq!(values(&span, DISTINCT_ID), ["user-1"]);
    assert_eq!(values(&span, SESSION_ID), ["session-1"]);
}
