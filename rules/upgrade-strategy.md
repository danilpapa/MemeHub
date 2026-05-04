# Overall Upgrade Strategy

Coordinated approach to upgrading MemeHub's distributed microservices system safely.

## Upgrade Categories

MemeHub has four independent upgrade domains that can be upgraded in any order:

1. **Rust Gateway Service** (`/gateway/Cargo.toml`)
2. **Python AI Service** (`/ai_service/requirements.txt`)
3. **Observability Stack** (`docker-compose.observability.yml`)
4. **CI/CD & Tooling** (`.github/workflows/`)

Each can be upgraded independently, but there are a few coordination points.

## Safe Upgrade Order

### Week 1-2: Foundation (Least Risk)
1. **CI/CD tooling** — Fastlane, Ruby, GitHub Actions
   - Risk: Low (isolated to CI, doesn't affect runtime)
   - Impact: Faster/better builds
2. **Docker infrastructure** — Base images, Docker Compose version
   - Risk: Low (but affects all services)
   - Impact: Security patches, performance

### Week 3: Core Services (Medium Risk)
3. **Python AI Service** — FastAPI, utility libraries
   - Risk: Medium (models may have compatibility issues)
   - Impact: Better features, security patches
4. **Rust Gateway** — Axum, web framework libraries
   - Risk: Medium (may require code changes)
   - Impact: Better performance, security patches

### Week 4: Observability (Can be any time)
5. **Observability Stack** — Prometheus, Grafana, Jaeger
   - Risk: Low (doesn't affect core functionality)
   - Impact: Better monitoring, fixes bugs in dashboards

## Pre-Upgrade Assessment

Before upgrading any component, answer these questions:

**Risk Assessment**:
- [ ] Are there known breaking changes in this release?
- [ ] Does this component have dependencies on others?
- [ ] Are there security vulnerabilities fixed in this version?
- [ ] How critical is this to production?

**Compatibility Check**:
- [ ] Do my dependencies still work together?
- [ ] Does the version support my OS/environment?
- [ ] Are there feature flags I need to enable?

**Testing Plan**:
- [ ] Can I test this locally?
- [ ] What metrics show success?
- [ ] How do I rollback if broken?

## Safe Upgrade Workflow (General)

### 1. Plan
```
Branch name: upgrade/component-version
Example: upgrade/fastlane-2.230.0
         upgrade/pytorch-2.4.0
         upgrade/grafana-10.5.0
```

### 2. Upgrade One Component at a Time
- Edit one dependency file
- Run: `make onboarding` or equivalent test
- Verify locally with: `docker compose up --build`
- Test API endpoints work
- Check logs for warnings

### 3. Commit Carefully
```bash
git add [files_changed]
git commit -m "upgrade(component): old-version → new-version

Description of changes:
- Feature X improved
- Breaking change Y requires Z

Testing:
- Local: make onboarding, docker compose up
- Verified: API endpoints respond, no warnings"
```

### 4. Push & Monitor CI/CD
```bash
git push origin upgrade/component-version
# Watch GitHub Actions
# All checks must pass (✓)
```

### 5. Create Pull Request
- Title: `upgrade(component): old-version → new-version`
- Description: See commit message + any test results
- Request review from team

### 6. Merge to Main
```bash
git checkout main
git pull origin main
git merge upgrade/component-version
git push origin main
```

## Coordinated Upgrade Example

Scenario: Upgrade all components in June 2026

**Week 1 (June 3-7)**
```
PR 1: upgrade(ci): fastlane 2.228 → 2.229
  - Test: fastlane build_ci passes
  - Merge: OK

PR 2: upgrade(infra): rust 1.80 → 1.81, python 3.12 → 3.13
  - Test: docker compose build succeeds
  - Merge: OK
```

**Week 2 (June 10-14)**
```
PR 3: upgrade(ai_service): fastapi 0.111 → 0.112, torch 2.3 → 2.4
  - Test: docker compose up ai_service, curl /ping
  - Merge: OK

PR 4: upgrade(gateway): axum 0.7 → 0.8, tokio 1.39 → 1.40
  - Test: docker compose up gateway, curl /health
  - Merge: OK
```

**Week 3 (June 17-21)**
```
PR 5: upgrade(observability): prometheus 2.52 → 2.53, grafana 10.4 → 10.5
  - Test: docker compose -f docker-compose.observability.yml up
  - Verify: traces in Jaeger, metrics in Prometheus
  - Merge: OK
```

## Dependency Relationships

```
┌─────────────────────────────────────────────────┐
│           Observability Stack                   │
│  (Jaeger, Prometheus, Grafana)                  │
│  ⚠️ Independent - can upgrade anytime          │
└─────────────────────────────────────────────────┘
                        ↑
         Depends on (weak) - gateway exports to Jaeger
                        ↑
┌─────────────────────────────────────────────────┐
│        Gateway Service (Rust)                   │
│  - Cargo.toml dependencies                      │
│  - Docker base image (rust:1.80-slim)           │
│  ⚠️ Cannot upgrade if AI Service broken        │
└─────────────────────────────────────────────────┘
         ↑
         Calls AI Service via HTTP
         ↑
┌─────────────────────────────────────────────────┐
│        AI Service (Python)                      │
│  - requirements.txt dependencies                │
│  - Docker base image (python:3.12-slim)         │
│  ⚠️ Most critical - model compatibility        │
└─────────────────────────────────────────────────┘
         ↑
         Called by Gateway
         ↑
┌─────────────────────────────────────────────────┐
│     CI/CD & Tooling (Fastlane, Ruby)            │
│  - .github/workflows/                           │
│  ⚠️ Only affects builds, not runtime            │
└─────────────────────────────────────────────────┘
```

## Critical Upgrade Rules

### Rule 1: Never Upgrade Multiple Components in One PR
❌ Bad:
```
PR: upgrade(all): 
  - Fastlane 2.228 → 2.229
  - Axum 0.7 → 0.8
  - PyTorch 2.3 → 2.4
  - Prometheus 2.52 → 2.53
```

✅ Good:
```
PR 1: upgrade(ci): fastlane 2.228 → 2.229
PR 2: upgrade(gateway): axum 0.7 → 0.8
PR 3: upgrade(ai_service): torch 2.3 → 2.4
PR 4: upgrade(observability): prometheus 2.52 → 2.53
```

**Why**: If any PR breaks, you know exactly which component and can rollback cleanly.

### Rule 2: Test Each Component in Isolation First
```bash
# Gateway only
docker compose build gateway
docker compose run gateway cargo test

# AI Service only
docker compose build ai_service
docker compose run ai_service python -m pytest

# Observability only
docker compose -f docker-compose.observability.yml up --build
```

Then test integration:
```bash
docker compose up --build
curl http://localhost:8080/health
```

### Rule 3: Document Breaking Changes in Commit Message
```
commit: upgrade(gateway): axum 0.7 → 0.8

BREAKING CHANGES:
- Router now requires explicit State layer
  → Updated app/router.rs to wrap State(app_state)
- Middleware composition syntax changed
  → Updated middleware/tracing.rs to use new format

Code changes: 3 files, 15 lines modified
Testing: cargo test, docker compose up, manual API test
```

### Rule 4: Keep CI/CD Separate from Dependency Upgrades
❌ Bad:
```
commit: chore: upgrade everything
  - Axum 0.7 → 0.8
  - Fastlane 2.228 → 2.229
  - PyTorch 2.3 → 2.4
```

✅ Good:
```
commit 1: upgrade(ci): fastlane 2.228 → 2.229
commit 2: upgrade(gateway): axum 0.7 → 0.8
commit 3: upgrade(ai_service): torch 2.3 → 2.4
```

**Why**: CI/CD changes should not interfere with debugging dependency issues.

### Rule 5: Always Test Before Pushing
```bash
# After editing dependency file
make onboarding  # Install/build everything
docker compose up --build
# Manual testing: curl endpoints, check logs
```

Only push after verification succeeds.

## Handling Breaking Changes

When you encounter breaking changes:

1. **Identify the change**:
   ```
   Error: Router::new requires State layer
   Caused by: axum 0.8 API change
   ```

2. **Find affected code**:
   ```bash
   grep -r "Router::new" gateway/src/
   ```

3. **Fix incrementally**:
   - Change one function at a time
   - Recompile with `cargo check`
   - Verify with `cargo test`

4. **Document in commit**:
   ```
   upgrade(gateway): axum 0.7 → 0.8
   
   Breaking change: Router now requires explicit State layer
   Updated: gateway/src/app/router.rs (lines 23-45)
   ```

## Monitoring After Upgrades

**After merging to main**, monitor for 24-48 hours:

```bash
# Check logs for errors
docker compose logs gateway ai_service
docker compose logs -f  # Follow logs

# Monitor metrics
# - Open http://localhost:9090 (Prometheus)
# - Check gateway_requests_total is increasing
# - Check no increase in error rates

# Monitor traces
# - Open http://localhost:16686 (Jaeger)
# - Verify recent traces from gateway
# - Check no span errors

# Monitor performance
# - Response times similar to before?
# - Memory usage reasonable?
# - CPU usage acceptable?
```

## Emergency Rollback

If production breaks after an upgrade:

```bash
# Revert the specific upgrade
git revert [upgrade_commit_hash]
git push origin main

# Or revert multiple commits
git revert HEAD~3..HEAD
git push origin main

# Rebuild with old version
docker compose down
docker system prune -a
docker compose up --build
```

**After rollback**:
1. Investigate root cause
2. Plan fix in new PR
3. Test thoroughly
4. Re-merge when ready

## Automated Dependency Checking

Consider adding tools for regular checks:

```bash
# Check for outdated dependencies regularly
# Rust
cargo outdated --root-only

# Python
pip list --outdated

# Docker images
docker pull <image>:latest
```

## Upgrade Calendar (Recommended)

- **Monthly**: Check for security updates, apply immediately
- **Quarterly** (every 3 months): Major feature upgrades, planned downtime
- **Annual**: Major version bumps (2.x → 3.x), significant refactoring

## When to Hold Back on Upgrades

⚠️ DO NOT upgrade if:
- [ ] Major version bump within 1 week of release (let bugs shake out)
- [ ] Breaking changes require significant code changes and you don't have time
- [ ] Dependency stability is not yet proven (< 1 month old)
- [ ] Your current version is working fine (only upgrade if benefit > risk)
- [ ] You don't have time to test thoroughly

✅ DO upgrade if:
- [ ] Security vulnerability announced
- [ ] Critical bug fix for your use case
- [ ] New feature you need
- [ ] Planned maintenance window

## Testing Checklist by Component

### Rust Gateway Upgrades
- [ ] `cargo check` completes
- [ ] `cargo test` passes
- [ ] `cargo build` succeeds
- [ ] `docker compose build gateway` works
- [ ] `curl http://localhost:8080/health` → 200
- [ ] `curl http://localhost:8080/api/...` returns expected data
- [ ] Logs show no warnings related to dependencies
- [ ] Jaeger shows traces from gateway

### Python AI Service Upgrades
- [ ] `pip install -r requirements.txt` succeeds
- [ ] `python -m pytest` passes (if tests exist)
- [ ] `docker compose build ai_service` works
- [ ] `curl http://localhost:8000/ping` → 200
- [ ] AI model loads without errors (or mocked in USE_LLM=0 mode)
- [ ] Logs show no deprecation warnings
- [ ] Gateway can call AI service endpoints

### Observability Upgrades
- [ ] All containers start: `docker compose -f docker-compose.observability.yml up`
- [ ] Prometheus targets healthy: `curl http://localhost:9090/api/v1/targets`
- [ ] Grafana datasources connected: `curl http://localhost:3000/api/datasources`
- [ ] Jaeger services appear: `curl http://localhost:16686/api/services`
- [ ] Generate traffic and verify metrics appear

### CI/CD Upgrades
- [ ] Fastlane command runs locally: `fastlane build_ci`
- [ ] GitHub Actions workflow file is valid YAML
- [ ] All checks pass on pushed branch (green ✓)
- [ ] No new warnings in CI logs

## Reference Links

- [Rust Gateway Upgrades](./rust-gateway-upgrades.md)
- [Python AI Service Upgrades](./python-ai-service-upgrades.md)
- [Observability Stack Upgrades](./observability-stack-upgrades.md)
- [Docker & Infrastructure Upgrades](./docker-infrastructure-upgrades.md)
- [CI/CD & Tooling Upgrades](./cicd-tooling-upgrades.md)
