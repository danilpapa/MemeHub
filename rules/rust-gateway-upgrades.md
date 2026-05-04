# Rust Gateway Upgrades

Managing Cargo.toml dependencies for the Axum-based gateway service.

**Location**: `/gateway/Cargo.toml`  
**Edition**: 2024  
**Current Key Dependencies**:
- Axum 0.7 (HTTP framework)
- Tokio 1 (async runtime)
- OpenTelemetry 0.27 (tracing)
- Metrics 0.23 (metrics collection)

## Safe Upgrade Workflow

### 1. Check for Updates
```bash
cd gateway
cargo update --dry-run
cargo outdated
```

### 2. Review Breaking Changes
- Check the changelog on crates.io for each dependency
- Pay special attention to:
  - **Axum**: HTTP middleware and routing changes
  - **Tokio**: Runtime API changes
  - **OpenTelemetry**: Tracing API and exporter changes
  - **Tower-HTTP**: Middleware layer changes

### 3. Update Specific Dependency
```bash
# Update a single crate to latest compatible version
cargo update -p axum

# Or edit Cargo.toml directly for specific versions
# Example: axum = "0.8"
```

### 4. Test Locally
```bash
# Compile and check for errors
cargo check

# Run tests if available
cargo test

# Build the Docker image
docker compose build gateway
```

### 5. Validate in Docker
```bash
# Run full stack locally
docker compose up --build

# Check logs for errors
docker compose logs gateway

# Test gateway is responding
curl http://localhost:8080/health
```

### 6. Commit Changes
```bash
git add gateway/Cargo.toml gateway/Cargo.lock
git commit -m "upgrade(gateway): update axum to 0.8

Breaking changes:
- Router now requires explicit State layer
- Middleware composition pattern updated

Testing: cargo test, docker compose up verified"
```

## Critical Dependencies & Compatibility

### Observability Tier (Must Stay in Sync)
These three must be compatible versions:
- `opentelemetry = "0.27"`
- `opentelemetry-otlp = "0.27"`
- `tracing = "0.1"`

**Upgrade rule**: Always update opentelemetry and opentelemetry-otlp to the same version.

### HTTP Framework Tier
- `axum = "0.7"` depends on `tower = "0.4"`
- `tower-http = "0.5"` provides middleware for Axum
- When updating Axum, verify tower-http compatibility

### Async Runtime
- `tokio = "1"` with `features = ["full"]`
- Very stable; breaking changes are rare
- Safe to update minor versions frequently

## Common Upgrade Scenarios

### Scenario A: Minor Axum Update (0.7 → 0.7.x)
✅ Safe, usually just bug fixes
```bash
cargo update -p axum
cargo test
```

### Scenario B: Major Axum Update (0.7 → 0.8)
⚠️ Review breaking changes, likely need code changes
1. Update Cargo.toml
2. Review Axum 0.8 migration guide
3. Update middleware in `gateway/src/middleware/`
4. Test thoroughly with `cargo test` and docker compose

### Scenario C: OpenTelemetry Update (0.27 → 0.28)
🔴 Requires coordination
1. Update opentelemetry and opentelemetry-otlp together
2. Check if tracing-opentelemetry also needs update
3. Test with `docker compose up` to verify Jaeger connection
4. Verify spans still appear in Jaeger UI

## Pre-Upgrade Checklist

- [ ] Current main branch is clean (`git status` is clean)
- [ ] All tests pass locally (`cargo test`)
- [ ] Docker image builds (`docker compose build gateway`)
- [ ] Gateway health check passes (`curl http://localhost:8080/health`)
- [ ] Review dependency changelogs
- [ ] Understand which APIs changed

## Post-Upgrade Validation

- [ ] `cargo check` completes without warnings
- [ ] `cargo test` passes
- [ ] `docker compose build gateway` succeeds
- [ ] `docker compose up` starts gateway without errors
- [ ] Logs show no panics or warnings related to dependencies
- [ ] Gateway responds to requests
- [ ] Tracing spans appear in Jaeger

## Rollback if Issues

```bash
# Revert to last known good state
git checkout gateway/Cargo.{toml,lock}
cargo build

# Or revert entire commit
git reset --soft HEAD~1
```
