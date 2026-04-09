use axum::{extract::State, response::Response, routing::get, Router};
use http_body_util::Full;
use prometheus::{
    Counter, Encoder, Gauge, Histogram, HistogramOpts, IntGauge, Registry, TextEncoder,
};
use std::sync::Arc;
use tokio::net::TcpListener;

#[cfg(feature = "metrics")]
#[derive(Clone)]
pub struct AgentMetrics {
    pub registry: Registry,

    // Legacy Metrics (kept for backward compatibility)
    pub active_rules: IntGauge,
    pub memory_usage_mb: IntGauge,
    pub connected_peers: IntGauge,
    pub rpc_calls_total: Counter,
    pub p2p_messages_total: Counter,
    pub behavioral_alerts_total: Counter,
    pub verification_failures_total: Counter,
    pub verification_duration_seconds: Histogram,

    // ERC-8004 Specific Metrics
    pub registry_registrations_total: Counter,
    pub registry_updates_total: Counter,

    pub validation_requests_received_total: Counter,
    pub validation_responses_submitted_total: Counter,
    pub validation_success_rate: Gauge,

    pub feedback_received_total: Counter,
    pub average_quality_score: Gauge,

    pub payments_received_total: Counter,
    pub payment_success_rate: Gauge,

    pub agent_uptime_seconds: Gauge,
    pub last_validation_timestamp: Gauge,
}

#[cfg(feature = "metrics")]
impl AgentMetrics {
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        // Legacy
        let active_rules = IntGauge::new("sods_active_rules", "Number of active monitoring rules")?;
        let memory_usage_mb = IntGauge::new("sods_memory_usage_mb", "Memory usage in MB")?;
        let connected_peers =
            IntGauge::new("sods_connected_peers", "Number of connected P2P peers")?;
        let rpc_calls_total = Counter::new("sods_rpc_calls_total", "Total RPC calls made")?;
        let p2p_messages_total =
            Counter::new("sods_p2p_messages_total", "Total P2P messages processed")?;
        let behavioral_alerts_total = Counter::new(
            "sods_behavioral_alerts_total",
            "Total behavioral alerts triggered",
        )?;
        let verification_failures_total = Counter::new(
            "sods_verification_failures_total",
            "Total failed block verifications",
        )?;
        let verification_duration_seconds = Histogram::with_opts(HistogramOpts::new(
            "sods_verification_duration_seconds",
            "Time spent verifying blocks",
        ))?;

        // ERC-8004
        let registry_registrations_total = Counter::new(
            "sods_registry_registrations_total",
            "Total agent registrations",
        )?;
        let registry_updates_total = Counter::new(
            "sods_registry_updates_total",
            "Total agent metadata updates",
        )?;
        let validation_requests_received_total = Counter::new(
            "sods_validation_requests_received_total",
            "Total validation requests received",
        )?;
        let validation_responses_submitted_total = Counter::new(
            "sods_validation_responses_submitted_total",
            "Total validation responses submitted",
        )?;
        let validation_success_rate = Gauge::new(
            "sods_validation_success_rate",
            "Current validation success rate percentage",
        )?;
        let feedback_received_total = Counter::new(
            "sods_feedback_received_total",
            "Total reputation feedback entries received",
        )?;
        let average_quality_score =
            Gauge::new("sods_average_quality_score", "Average agent quality score")?;
        let payments_received_total = Counter::new(
            "sods_payments_received_total",
            "Total escrow payments received",
        )?;
        let payment_success_rate = Gauge::new(
            "sods_payment_success_rate",
            "Escrow payment success rate percentage",
        )?;
        let agent_uptime_seconds =
            Gauge::new("sods_agent_uptime_seconds", "Agent uptime in seconds")?;
        let last_validation_timestamp = Gauge::new(
            "sods_last_validation_timestamp",
            "Unix timestamp of the last validation",
        )?;

        // Register all
        registry.register(Box::new(active_rules.clone()))?;
        registry.register(Box::new(memory_usage_mb.clone()))?;
        registry.register(Box::new(connected_peers.clone()))?;
        registry.register(Box::new(rpc_calls_total.clone()))?;
        registry.register(Box::new(p2p_messages_total.clone()))?;
        registry.register(Box::new(behavioral_alerts_total.clone()))?;
        registry.register(Box::new(verification_failures_total.clone()))?;
        registry.register(Box::new(verification_duration_seconds.clone()))?;

        registry.register(Box::new(registry_registrations_total.clone()))?;
        registry.register(Box::new(registry_updates_total.clone()))?;
        registry.register(Box::new(validation_requests_received_total.clone()))?;
        registry.register(Box::new(validation_responses_submitted_total.clone()))?;
        registry.register(Box::new(validation_success_rate.clone()))?;
        registry.register(Box::new(feedback_received_total.clone()))?;
        registry.register(Box::new(average_quality_score.clone()))?;
        registry.register(Box::new(payments_received_total.clone()))?;
        registry.register(Box::new(payment_success_rate.clone()))?;
        registry.register(Box::new(agent_uptime_seconds.clone()))?;
        registry.register(Box::new(last_validation_timestamp.clone()))?;

        let metrics = Self {
            registry,
            active_rules,
            memory_usage_mb,
            connected_peers,
            rpc_calls_total,
            p2p_messages_total,
            behavioral_alerts_total,
            verification_failures_total,
            verification_duration_seconds,
            registry_registrations_total,
            registry_updates_total,
            validation_requests_received_total,
            validation_responses_submitted_total,
            validation_success_rate,
            feedback_received_total,
            average_quality_score,
            payments_received_total,
            payment_success_rate,
            agent_uptime_seconds,
            last_validation_timestamp,
        };

        // Start uptime tracking
        let start_time = std::time::Instant::now();
        let uptime_metrics = metrics.clone();
        tokio::spawn(async move {
            loop {
                let uptime = start_time.elapsed().as_secs();
                // Since agent_uptime_seconds is a Counter, we can't reliably "set" it
                // easily with Prometheus standard Counter unless we use its internal value or increment.
                // However, our health check needs total uptime.
                // Let's use inc_by(1) every second for simplicity if we want to keep it a counter,
                // or just change it to a Gauge if we want to "set" it.
                // The user's requirement says .set(uptime), so I will change it back to a Gauge in the definition.
                uptime_metrics.agent_uptime_seconds.set(uptime as f64);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });

        Ok(metrics)
    }

    pub async fn start_http_server(self: Arc<Self>, port: u16) {
        let mut app = Router::new();

        #[cfg(feature = "api")]
        {
            app = app.merge(crate::api::health::router::<Arc<AgentMetrics>>());
        }

        let app = app
            .route(
                "/_metrics",
                get(|State(m): State<Arc<AgentMetrics>>| async move {
                    let encoder = TextEncoder::new();
                    let metric_families = m.registry.gather();
                    let mut buffer = Vec::new();
                    encoder.encode(&metric_families, &mut buffer).unwrap();

                    Response::builder()
                        .header("Content-Type", encoder.format_type())
                        .body(Full::from(buffer))
                        .unwrap()
                }),
            )
            .with_state(self);

        let addr = format!("0.0.0.0:{}", port);
        let listener = match TcpListener::bind(&addr).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Metrics Server Error: Failed to bind to {}: {}", addr, e);
                return;
            }
        };

        println!("📈 Metrics Server: listening on http://{}/_metrics", addr);
        if let Err(e) = axum::serve(listener, app).await {
            eprintln!("Metrics Server Error: {}", e);
        }
    }
}
