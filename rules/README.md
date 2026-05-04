# MemeHub Upgrade Rules

This directory contains guidelines for safely upgrading different components of the MemeHub microservices architecture.

## Quick Reference

- **[Rust Gateway Upgrades](./rust-gateway-upgrades.md)** — Axum, Tokio, OpenTelemetry dependencies
- **[Python AI Service Upgrades](./python-ai-service-upgrades.md)** — FastAPI, PyTorch, Transformers versions
- **[Observability Stack Upgrades](./observability-stack-upgrades.md)** — Jaeger, Prometheus, Grafana containers
- **[Docker & Infrastructure Upgrades](./docker-infrastructure-upgrades.md)** — Docker Compose, base images
- **[CI/CD & Tooling Upgrades](./cicd-tooling-upgrades.md)** — Fastlane, GitHub Actions, Ruby versions
- **[Overall Upgrade Strategy](./upgrade-strategy.md)** — Safe patterns for coordinated upgrades

## Project Architecture Overview

MemeHub is a containerized microservices system:
- **Gateway Service**: Rust/Axum HTTP server with OTLP tracing (port 8080)
- **AI Service**: Python/FastAPI OCR & emotion detection (port 8000)
- **Observability**: Jaeger (tracing), Prometheus (metrics), Grafana (dashboards)
- **Orchestration**: Docker Compose for local/CI environments
- **CI/CD**: Fastlane 2.228.0 + GitHub Actions

## Upgrade Readiness Checklist

Before upgrading any component:
- [ ] Check current version in relevant manifest (Cargo.toml, requirements.txt, docker-compose.yml)
- [ ] Review changelog for breaking changes
- [ ] Test locally with `make onboarding` and `make container`
- [ ] Run CI/CD validation before pushing
- [ ] Update lock files (Cargo.lock for Rust, pip-freeze for Python)
- [ ] Document breaking changes in your commit message

## Current Versions (as of May 2026)

| Component | Current Version | Lock File |
|-----------|-----------------|-----------|
| Rust Toolchain | 2024 Edition | — |
| Axum | 0.7 | Cargo.lock |
| FastAPI | 0.111.0 | requirements.txt |
| PyTorch | 2.3.1 | requirements.txt |
| Transformers | 4.41.2 | requirements.txt |
| Jaeger | 1.59 | docker-compose.yml |
| Prometheus | v2.52.0 | docker-compose.observability.yml |
| Grafana | 10.4.2 | docker-compose.observability.yml |
| Fastlane | 2.228.0 | .github/workflows/build-on-main.yml |
