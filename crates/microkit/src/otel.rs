use crate::config::OtelConfig;
use axum::Router;
use axum_otel::{AxumOtelOnFailure, AxumOtelOnResponse, AxumOtelSpanCreator};
use axum_otel_metrics::HttpMetricsLayerBuilder;
use opentelemetry::global;
use opentelemetry_otlp::{MetricExporter, SpanExporter, WithHttpConfig};
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::{Resource, propagation::TraceContextPropagator};
use std::collections::HashMap;
use tower_http::trace::TraceLayer;

// use opentelemetry_otlp::LogExporter;
// use opentelemetry_sdk::logs::SdkLoggerProvider;
// use opentelemetry_appender_log::OpenTelemetryLogBridge;

pub fn init(router: Router, service_name: &str, config: &Option<OtelConfig>) -> Router {
    if config.is_none() {
        return router;
    }

    let url = &config.as_ref().unwrap().url;
    let token = &config.as_ref().unwrap().token;

    let mut map = HashMap::new();
    map.insert("Authorization".to_string(), format!("Api-Token {}", token));
    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .build();

    // Tracing
    global::set_text_map_propagator(TraceContextPropagator::new());
    let tracer_exporter = SpanExporter::builder()
        .with_http()
        .with_headers(map.clone())
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(format!("{}/v1/traces", url))
        .build()
        .unwrap();
    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(tracer_exporter)
        .build();
    global::set_tracer_provider(tracer_provider.clone());

    // Metrics
    let metrics_exporter = MetricExporter::builder()
        .with_http()
        .with_headers(map.clone())
        .with_endpoint(format!("{}/v1/metrics", url))
        .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
        .build()
        .unwrap();
    let meter_provider = SdkMeterProvider::builder()
        .with_reader(PeriodicReader::builder(metrics_exporter).build())
        .with_resource(resource.clone())
        .build();
    let metrics = HttpMetricsLayerBuilder::new().build();
    global::set_meter_provider(meter_provider);

    // Logs
    // let logger_exporter = LogExporter::builder()
    //     .with_http()
    //     .with_headers(map.clone())
    //     .with_endpoint(otel_url.clone() + "/v1/logs")
    //     .with_protocol(opentelemetry_otlp::Protocol::HttpBinary)
    //     .build()
    //     .unwrap();
    // let logger_provider = SdkLoggerProvider::builder()
    //     .with_batch_exporter(logger_exporter)
    //     .with_resource(resource.clone())
    //     .build();
    // let otel_log_appender = OpenTelemetryLogBridge::new(&logger_provider);
    //
    // log::set_boxed_logger(Box::new(otel_log_appender)).unwrap();
    // log::set_max_level(log::LevelFilter::Debug);

    router
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(AxumOtelSpanCreator::new().level(tracing::Level::INFO))
                .on_response(AxumOtelOnResponse::new().level(tracing::Level::INFO))
                .on_failure(AxumOtelOnFailure::new()),
        )
        .layer(metrics)
}
