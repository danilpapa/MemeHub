# MemeHub — Microservices Infrastructure for Meme Analysis

> Event-driven microservices platform with Rust gateway, Python AI service, and full observability stack.

## Documentation

- **[Final.md](./Final.md)** — Complete architecture guide (Russian) with all components, storage layers, observability, Kubernetes setup, and problem-solving stories
- **[danilpapa.md](./danilpapa.md)** — Educational guide for iOS developers (Russian) learning infrastructure
- **[rules/](./rules/)** — Upgrade guidelines for dependencies and infrastructure

## Quick Start

### Development (Docker Compose)
```bash
# Basic stack
docker compose up --build -d

# With observability (Jaeger, Prometheus, Grafana)
docker compose -f docker-compose.yml -f docker-compose.observability.yml up --build -d

# Check health
curl http://localhost:8080/metrics
```

### Production (Kubernetes)
```bash
cd terraform
terraform init
terraform plan
terraform apply -auto-approve
kubectl get all -n memehub
```

## Architecture at a Glance

**Services:**
- **Gateway** (Rust/Axum): HTTP API, Kafka producer/consumer, job management
- **AI Service** (Python/FastAPI): OCR processing, emotion classification, analytics sink
- **Event Bus** (Kafka/Redpanda): Async task queue between services
- **Data Layers**: Redis (hot), PostgreSQL (cold), MinIO (objects), ClickHouse (analytics)
- **Observability**: Jaeger (tracing), Prometheus (metrics), Grafana (dashboards)

**Key Flow:**
```
Client → Gateway → MinIO (upload) → Kafka (request event) → AI Service → ClickHouse
   ↑                                                              ↓
   └─ Redis + PostgreSQL (status/results) ← Kafka (completion event) ←
```

## Important Files

```
.
├── gateway/                          # Rust HTTP server
│   ├── Cargo.toml                   # Dependencies
│   ├── src/
│   │   ├── main.rs                  # Entry point
│   │   ├── app/                     # Router & middleware
│   │   ├── handlers/                # HTTP endpoints
│   │   ├── services/                # Redis, Kafka, MinIO, PostgreSQL
│   │   └── observability/           # Tracing & metrics
│   ├── migrations/                  # SQL migrations
│   └── Dockerfile                   # Multi-stage build
├── ai_service/                       # Python AI processor
│   ├── main.py                      # FastAPI app
│   ├── requirements.txt             # Dependencies
│   └── Dockerfile
├── docker-compose.yml               # Main stack
├── docker-compose.observability.yml # Observability add-on
├── terraform/                        # Kubernetes IaC
│   ├── main.tf                      # Provider config
│   ├── namespaces.tf                # K8s namespaces
│   ├── service_accounts.tf          # RBAC setup
│   ├── secrets.tf                   # Encrypted credentials
│   └── variables.tf                 # Input variables
├── k8s/                             # Kubernetes manifests
│   └── 10-ingress-controller.yaml   # Nginx Ingress + rate limiting
├── rules/                           # Upgrade guidelines
├── Final.md                         # Complete architecture (this is key!)
└── danilpapa.md                     # Educational guide
```

## Ports Reference

| Service | Port | Purpose |
|---------|------|---------|
| Gateway | 8080 | HTTP API + `/metrics` |
| AI Service | 8000 | Health check + `/process` endpoint |
| Redis | 6379 | Hot storage for job status |
| PostgreSQL | 5432 | Cold storage for results |
| MinIO API | 9000 | Object storage |
| MinIO Console | 9001 | Web UI |
| ClickHouse | 8123 | Analytics database |
| Jaeger UI | 16686 | Distributed tracing |
| Prometheus | 9090 | Metrics database |
| Grafana | 3001 | Dashboards (admin/admin) |
| Kafka | 9092 | Internal, 19092 from host |

## Environment Setup

### For Kubernetes/Terraform
```bash
# Install Terraform
brew tap hashicorp/tap
brew install hashicorp/tap/terraform

# Initialize
cd terraform
terraform init
```

### For Local Development
```bash
# Requires Docker Desktop, Make, Rust
make onboarding  # Setup script
make container   # Run via servicectl
```

## Next Steps

1. **Learn architecture**: Read `Final.md` for comprehensive overview
2. **Understand components**: Each `rules/` file covers safe upgrade patterns
3. **Deploy locally**: Use Docker Compose for development
4. **Deploy to K8s**: Use Terraform for production setup
5. **Monitor**: Access Jaeger/Prometheus/Grafana for observability
