#![allow(dead_code)]

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracer, SdkTracerProvider, SpanData};
use posthog_rs::{Client, ClientOptionsBuilder, RequestContext};
use posthog_rs_otel::PostHogContextSpanProcessor;
use tracing_subscriber::layer::SubscriberExt;

pub struct Recorder {
    pub provider: SdkTracerProvider,
    pub exporter: InMemorySpanExporter,
}

impl Recorder {
    pub fn new(client: &Client) -> Self {
        let exporter = InMemorySpanExporter::default();
        let provider = SdkTracerProvider::builder()
            .with_span_processor(PostHogContextSpanProcessor::new(client))
            .with_simple_exporter(exporter.clone())
            .build();
        Self { provider, exporter }
    }

    pub fn tracer(&self) -> SdkTracer {
        self.provider.tracer("posthog-rs-otel-test")
    }

    /// Routes `tracing` spans on this thread to the recorder until the guard drops.
    pub fn install_tracing(&self) -> tracing::subscriber::DefaultGuard {
        let otel_layer = tracing_opentelemetry::layer().with_tracer(self.tracer());
        let subscriber = tracing_subscriber::registry().with(otel_layer);
        tracing::subscriber::set_default(subscriber)
    }

    pub fn span(&self, name: &str) -> SpanData {
        let spans = self.exporter.get_finished_spans().expect("finished spans");
        spans
            .into_iter()
            .find(|span| span.name == name)
            .unwrap_or_else(|| panic!("no finished span named {name}"))
    }
}

/// Every value the span has for `key`, in order. More than one means the key
/// was set twice.
pub fn values(span: &SpanData, key: &str) -> Vec<String> {
    span.attributes
        .iter()
        .filter(|attribute| attribute.key.as_str() == key)
        .map(|attribute| attribute.value.to_string())
        .collect()
}

pub async fn enabled_client() -> Client {
    client_with(false).await
}

pub async fn disabled_client() -> Client {
    client_with(true).await
}

async fn client_with(disabled: bool) -> Client {
    let options = ClientOptionsBuilder::default()
        .api_key("phc_test".to_string())
        .host("http://127.0.0.1:9")
        .disabled(disabled)
        .build()
        .expect("client options");
    posthog_rs::client(options).await
}

pub fn request(distinct_id: &str, session_id: &str) -> RequestContext {
    RequestContext::new()
        .distinct_id(distinct_id)
        .session_id(session_id)
}
