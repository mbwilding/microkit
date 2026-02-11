use crate::config::OtelConfig;
use anyhow::{Context, Result, bail};
use axum::Router;
use axum_otel::{AxumOtelOnFailure, AxumOtelOnResponse, AxumOtelSpanCreator};
use axum_otel_metrics::HttpMetricsLayerBuilder;
use opentelemetry::global;
use opentelemetry_appender_log::OpenTelemetryLogBridge;
use opentelemetry_otlp::{LogExporter, MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::logs::SdkLoggerProvider;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::{Resource, propagation::TraceContextPropagator};
use tower_http::trace::TraceLayer;

// TODO: Get token hooked up to OTEL
pub fn init_providers(
    service_name: &str,
    config: &Option<OtelConfig>,
) -> Result<Option<SdkTracerProvider>> {
    if config.is_none() {
        bail!("otel: init_providers called but no config found");
    }

    let url = match config.as_ref() {
        Some(cfg) => &cfg.url,
        None => bail!("otel: init_providers called but no otel.url was set"),
    };

    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .build();

    global::set_text_map_propagator(TraceContextPropagator::new());

    let tracer_exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(url.clone())
        .build()
        .context("Failed to create tracer exporter")?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(tracer_exporter)
        .build();

    global::set_tracer_provider(tracer_provider.clone());

    let metrics_exporter = MetricExporter::builder()
        .with_tonic()
        .with_endpoint(url.clone())
        .build()
        .context("Failed to create metrics exporter")?;

    let meter_provider = SdkMeterProvider::builder()
        .with_reader(PeriodicReader::builder(metrics_exporter).build())
        .with_resource(resource.clone())
        .build();
    global::set_meter_provider(meter_provider);

    let logger_exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(url.clone())
        .build()
        .context("Failed to create log exporter")?;
    let logger_provider = SdkLoggerProvider::builder()
        .with_batch_exporter(logger_exporter)
        .with_resource(resource.clone())
        .build();
    let otel_log_appender = OpenTelemetryLogBridge::new(&logger_provider);

    log::set_boxed_logger(Box::new(otel_log_appender)).context("Failed to set logger")?;
    log::set_max_level(log::LevelFilter::Info);

    Ok(Some(tracer_provider))
}

pub fn apply_layers(router: Router) -> Router {
    let metrics = HttpMetricsLayerBuilder::new().build();

    router
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(AxumOtelSpanCreator::new().level(tracing::Level::INFO))
                .on_response(AxumOtelOnResponse::new().level(tracing::Level::INFO))
                .on_failure(AxumOtelOnFailure::new()),
        )
        .layer(metrics)
}
