# Production Monitoring for SODS Agents

This guide describes how to set up and use the monitoring and alerting infrastructure for SODS agents operating in production environments.

## Infrastructure Requirements

To fully utilize the SODS monitoring stack, you should have the following tools deployed:
- **Prometheus** (v2.30+): For metrics collection and storage.
- **Grafana** (v9.0+): For visualization and dashboards.
- **Alertmanager** (v0.25+): For alert routing and notifications.

## Metrics Reference

The SODS agent exposes metrics in Prometheus format at `/_metrics`.

| Metric | Description | Type |
|--------|-------------|------|
| `sods_registry_registrations_total` | Total ERC-8004 agent registrations | Counter |
| `sods_validation_requests_received_total` | Total requests received for verification | Counter |
| `sods_validation_success_rate` | Current validation success percentage | Gauge |
| `sods_average_quality_score` | Reputation score calculated from feedback | Gauge |
| `sods_agent_uptime_seconds` | Total uptime of the agent process | Gauge |
| `sods_payments_received_total` | Successful payments received from escrow | Counter |

## Setup Instructions

### 1. Prometheus Configuration

Add the SODS agent as a scrape target in your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'sods-agent'
    static_configs:
      - targets: ['localhost:8080'] # Update with your agent's API port
```

### 2. Alerting Configuration

Import the alerting rules provided in `config/alerts.yaml` into your Prometheus configuration.

### 3. Grafana Dashboard

Import the dashboard template located at `dashboards/sods-agent.json` into your Grafana instance to get a real-time overview of your agent's performance.

## Health Checks

The agent provides standardized health endpoints for automated orchestration (e.g., Kubernetes Liveness/Readiness):

- **GET /health**: Returns the overall status, registry connectivity, and key metrics.
- **GET /health/ready**: Returns HTTP 200 if the agent is ready to process requests.

## Structured Logging

Logs are output in structured JSON format by default when running in production. This allows for easy aggregation in tools like **Grafoki** or **ELK**.

```json
{"timestamp":"2026-02-01T12:00:00Z","level":"INFO","event":"validation_completed","agent_id":"0x...","result":"success","duration_ms":120}
```
