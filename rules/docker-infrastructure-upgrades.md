# Docker & Infrastructure Upgrades

Managing Docker, Docker Compose, base images, and infrastructure versioning.

**Locations**:
- `/gateway/Dockerfile` — Rust gateway base image
- `/ai_service/Dockerfile` — Python AI service base image
- `docker-compose.yml` — Docker Compose version and service configuration
- `docker-compose.observability.yml` — Observability services configuration

## Components to Upgrade

### 1. Docker Desktop/Engine

Check your Docker version:
```bash
docker --version
docker compose version
```

**Current recommended**: Docker 25.x with Docker Compose v2.20+

**Upgrade process**:
- macOS: Download latest from Docker Desktop
- Linux: Use package manager or official docs
- Windows: Download Docker Desktop installer

**After upgrade**:
```bash
docker --version
docker compose --version
docker compose up --build  # Verify everything still works
```

### 2. Base Images (Dockerfile)

**Gateway base image** (`/gateway/Dockerfile`):
```dockerfile
FROM rust:1.80-slim
# Current: rust:1.80-slim
# Check for newer: rust:1.81-slim, rust:1.82-slim
```

**AI Service base image** (`/ai_service/Dockerfile`):
```dockerfile
FROM python:3.12-slim
# Current: python:3.12-slim
# Check for: python:3.13-slim (if compatible with dependencies)
```

#### Safe Upgrade Workflow for Base Images

1. **Check compatibility**:
   - Rust: Check if newer Rust Edition is available
   - Python: Check if newer Python version works with PyTorch/Transformers

2. **Update single image**:
   ```bash
   # Edit gateway/Dockerfile
   # FROM rust:1.80-slim → FROM rust:1.81-slim
   
   # Build and test
   docker compose build gateway
   docker compose up gateway
   ```

3. **Test in container**:
   ```bash
   # For Rust gateway
   docker compose run gateway cargo --version
   docker compose run gateway rustc --version
   
   # For Python AI service
   docker compose run ai_service python --version
   docker compose run ai_service pip list
   ```

4. **Full integration test**:
   ```bash
   docker compose up --build
   curl http://localhost:8080/health
   curl http://localhost:8000/ping
   ```

### 3. Docker Compose Version

**Location**: `docker-compose.yml` top line `version:`

```yaml
version: '3.8'
# Current: 3.8 (supports features up to Docker Engine 19.03+)
# Can upgrade to: 3.9, 3.10, 3.11, 3.12 (latest)
```

**Compatibility table**:
| Compose Version | Docker Engine | Features |
|-----------------|---------------|----------|
| 3.8 | 19.03+ | Secrets, configs, resource limits |
| 3.9 | 20.10+ | Service profiles |
| 3.10+ | 20.10+ | Additional validation |

**Safe upgrade**:
1. Edit `version: '3.8'` → `version: '3.12'`
2. Run: `docker compose up --build`
3. If errors, review Docker Compose release notes and check for incompatible features

**Rarely needed**: Docker Compose v3 is very stable and widely compatible

### 4. Multi-Stage Build Optimization

**Current gateway Dockerfile**:
```dockerfile
FROM rust:1.80-slim as builder
# Build stage

FROM debian:bookworm-slim
# Runtime stage (much smaller)
```

**Upgrade opportunity**: Use builder pattern for Python too if disk space is concern

### 5. Docker Compose Services Configuration

**Health checks** (already configured):
```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
  interval: 30s
  timeout: 5s
  retries: 3
```

These are stable and rarely need updates.

## Common Upgrade Scenarios

### Scenario A: Minor Rust Update (1.80 → 1.81)
✅ Safe, almost always compatible
```bash
# Edit gateway/Dockerfile
sed -i '' 's/FROM rust:1.80-slim/FROM rust:1.81-slim/' gateway/Dockerfile

# Build and test
docker compose build gateway
docker compose run gateway cargo --version
```

### Scenario B: Minor Python Update (3.12 → 3.13)
⚠️ Verify PyTorch/Transformers compatibility first
```bash
# Check if PyTorch supports Python 3.13
# https://pytorch.org/get-started/locally/

# Only upgrade if confirmed compatible
sed -i '' 's/FROM python:3.12-slim/FROM python:3.13-slim/' ai_service/Dockerfile

# Build and test thoroughly
docker compose build ai_service
docker compose up ai_service
```

### Scenario C: Docker Compose Version (3.8 → 3.12)
✅ Very safe, backward compatible
```bash
# Edit docker-compose.yml and docker-compose.observability.yml
sed -i '' 's/version: .3.8./version: "3.12"/' docker-compose.yml
sed -i '' 's/version: .3.8./version: "3.12"/' docker-compose.observability.yml

# Test
docker compose up --build
```

## Pre-Upgrade Checklist

- [ ] Current Docker version: `docker --version`
- [ ] Current Docker Compose version: `docker compose version`
- [ ] All containers running cleanly: `docker compose up` succeeds
- [ ] Review base image changelog (Rust, Python, Debian)
- [ ] Check compatibility matrix for major versions
- [ ] Have backup of Dockerfile versions (git will handle this)

## Post-Upgrade Validation

- [ ] Base image builds: `docker compose build --no-cache`
- [ ] Containers start: `docker compose up --build`
- [ ] Health checks pass: `docker compose ps` shows "healthy"
- [ ] Services respond: `curl http://localhost:8080/health`
- [ ] No deprecation warnings in logs
- [ ] Image sizes reasonable (no unexpected bloat)

## Disk Space Management

After multiple upgrades, clean up old images:
```bash
# Remove unused images
docker image prune -a

# Remove unused volumes
docker volume prune

# Check disk usage
docker system df

# Full cleanup (be careful!)
docker system prune -a --volumes
```

## Environment Parity

Ensure local Docker matches CI/CD expectations:

**GitHub Actions** (from `.github/workflows/build-on-main.yml`):
- Uses latest stable Rust via `rustup`
- Uses GitHub Actions default Docker version
- Runs: `docker compose build`

**Local Development**:
- Use `docker --version` to check your version
- If >2 minor versions behind, consider updating

## Base Image Security Updates

Schedule regular checks for security updates:
```bash
# Check what's new in base images
docker pull rust:latest
docker inspect rust:latest

docker pull python:latest
docker inspect python:latest

# If patch version has security update:
# rust:1.80.0 → rust:1.80.1
# Update Dockerfile and rebuild
```

## Rollback if Issues

```bash
# Revert Dockerfile or compose file
git checkout gateway/Dockerfile ai_service/Dockerfile docker-compose.yml

# Rebuild with old versions
docker compose down
docker image prune -a
docker compose build --no-cache
```

## Build Time Optimization

If upgrades make builds slower:

1. **Use BuildKit** (faster builds):
   ```bash
   DOCKER_BUILDKIT=1 docker compose build
   ```

2. **Check for larger base images**:
   ```bash
   # Compare image sizes
   docker images | grep -E 'rust|python'
   ```

3. **Consider caching strategy**:
   - Use `.dockerignore` to exclude unnecessary files
   - Order Dockerfile layers from slowest-to-change to fastest

## Network & Volumes Configuration

Usually stable across versions, but verify after major upgrades:
```bash
# Check networks
docker network ls
docker network inspect memehub_default

# Check volumes
docker volume ls
docker volume inspect memehub_ai_models (if exists)
```

## Documentation References

- **Docker Official**: https://docs.docker.com/engine/release-notes/
- **Docker Compose**: https://docs.docker.com/compose/release-notes/
- **Rust Docker**: https://hub.docker.com/_/rust
- **Python Docker**: https://hub.docker.com/_/python
