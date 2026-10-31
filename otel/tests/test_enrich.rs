mod support;

use posthog_rs_otel::enrich_current_span;
use support::{disabled_client, enabled_client, request, values, Recorder};
use tracing::Instrument;

const DISTINCT_ID: &str = "posthogDistinctId";
const SESSION_ID: &str = "sessionId";

#[tokio::test]
async fn covers_a_span_that_started_before_the_request_context() {
    let client = enabled_client().await;
    let recorder = Recorder::new(&client);
    let _tracing = recorder.install_tracing();

    let handle_request = request("user-1", "session-1").scope(async {
        enrich_current_span(&client);
    });
    handle_request
        .instrument(tracing::info_span!("request"))
        .await;

    let span = recorder.span("request");
    assert_eq!(values(&span, DISTINCT_ID), ["user-1"]);
    assert_eq!(values(&span, SESSION_ID), ["session-1"]);
}

#[tokio::test]
async fn keeps_a_field_the_span_declares() {
    let client = enabled_client().await;
    let recorder = Recorder::new(&client);
    let _tracing = recorder.install_tracing();

    let handle_request = request("header-user", "session-1").scope(async {
        enrich_current_span(&client);
    });
    handle_request
        .instrument(tracing::info_span!(
            "request",
            posthogDistinctId = "app-user"
        ))
        .await;

    let span = recorder.span("request");
    assert_eq!(values(&span, DISTINCT_ID), ["app-user"]);
    assert_eq!(values(&span, SESSION_ID), ["session-1"]);
}

#[tokio::test]
async fn adds_nothing_outside_a_request() {
    let client = enabled_client().await;
    let recorder = Recorder::new(&client);
    let _tracing = recorder.install_tracing();

    tracing::info_span!("startup").in_scope(|| enrich_current_span(&client));

    assert!(values(&recorder.span("startup"), DISTINCT_ID).is_empty());
}

#[tokio::test]
async fn adds_nothing_for_a_disabled_client() {
    let client = disabled_client().await;
    let recorder = Recorder::new(&client);
    let _tracing = recorder.install_tracing();

    let handle_request = request("user-1", "session-1").scope(async {
        enrich_current_span(&client);
    });
    handle_request
        .instrument(tracing::info_span!("request"))
        .await;

    assert!(values(&recorder.span("request"), DISTINCT_ID).is_empty());
}
