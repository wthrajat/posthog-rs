//! A page with posthog-js calls a traced axum backend. Each backend trace in
//! PostHog Tracing links to the person and the session replay that sent it.
//!
//! ```bash
//! export POSTHOG_API_TOKEN=phc_your_project_token
//! export POSTHOG_HOST=https://us.i.posthog.com   # optional
//! cargo run -p posthog-rs-otel --example axum_checkout
//! ```
//!
//! Open http://localhost:3000, click "Check out" a few times, then press
//! Ctrl+C to flush. Open Tracing in PostHog and pick service `axum-checkout`.

use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use posthog_rs::tower::PostHogLayer;
use posthog_rs::RequestContext;
use posthog_rs_otel::PostHogContextSpanProcessor;
use tower_http::trace::TraceLayer;
use tracing::Instrument;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

const ADDRESS: &str = "127.0.0.1:3000";

struct Settings {
    token: String,
    host: String,
}

impl Settings {
    fn from_env() -> Self {
        let token = std::env::var("POSTHOG_API_TOKEN").expect("set POSTHOG_API_TOKEN");
        let host = std::env::var("POSTHOG_HOST")
            .unwrap_or_else(|_| "https://us.i.posthog.com".to_string());
        Self { token, host }
    }
}

#[tokio::main]
async fn main() {
    let settings = Settings::from_env();
    let client = Arc::new(posthog_client(&settings).await);
    let provider = tracer_provider(&settings, &client);
    install_tracing(&provider);

    let listener = tokio::net::TcpListener::bind(ADDRESS).await.unwrap();
    println!("Open http://localhost:3000 and press Ctrl+C to flush and exit");
    axum::serve(listener, app(client.clone(), page(&settings)))
        .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
        .await
        .unwrap();

    client.flush().await;
    tokio::task::spawn_blocking(move || provider.shutdown())
        .await
        .unwrap()
        .unwrap();
}

fn app(client: Arc<posthog_rs::Client>, page: String) -> Router {
    Router::new()
        .route("/", get(move || async move { Html(page) }))
        .route("/checkout", get(checkout))
        .with_state(client)
        // PostHogLayer goes outside TraceLayer, so the request span starts
        // inside the request context and gets the IDs too.
        .layer(TraceLayer::new_for_http())
        .layer(PostHogLayer::new())
}

async fn checkout(State(client): State<Arc<posthog_rs::Client>>) -> &'static str {
    tokio::time::sleep(Duration::from_millis(40))
        .instrument(tracing::info_span!("db.load_cart"))
        .await;
    tokio::time::sleep(Duration::from_millis(250))
        .instrument(tracing::info_span!("payments.charge"))
        .await;

    let context = RequestContext::current().unwrap_or_default();
    client.capture(context.event("checkout_completed"));
    "Order placed"
}

async fn posthog_client(settings: &Settings) -> posthog_rs::Client {
    let options = posthog_rs::ClientOptionsBuilder::default()
        .api_key(settings.token.clone())
        .host(settings.host.as_str())
        .build()
        .unwrap();
    posthog_rs::client(options).await
}

fn tracer_provider(settings: &Settings, client: &posthog_rs::Client) -> SdkTracerProvider {
    let exporter = SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpJson)
        .with_endpoint(format!("{}/i/v1/traces", settings.host))
        .with_headers(
            [(
                "Authorization".to_string(),
                format!("Bearer {}", settings.token),
            )]
            .into(),
        )
        .build()
        .unwrap();
    let resource = Resource::builder()
        .with_service_name("axum-checkout")
        .build();

    SdkTracerProvider::builder()
        .with_span_processor(PostHogContextSpanProcessor::new(client))
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build()
}

fn install_tracing(provider: &SdkTracerProvider) {
    use opentelemetry::trace::TracerProvider as _;

    let tracer = provider.tracer("axum-checkout");
    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();
}

/// posthog-js sends `X-POSTHOG-DISTINCT-ID` and `X-POSTHOG-SESSION-ID` on
/// requests to hosts listed in `tracing_headers`.
fn page(settings: &Settings) -> String {
    include_str!("checkout.html")
        .replace("{token}", &settings.token)
        .replace("{host}", &settings.host)
}
