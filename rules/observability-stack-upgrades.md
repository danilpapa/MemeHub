# Observability Stack Upgrades

Managing container versions for Jaeger, Prometheus, and Grafana services.

**Locations**:
- Jaeger: `docker-compose.yml` (line: `image: jaegertracing/all-in-one:1.59`)
- Prometheus: `docker-compose.observability.yml` (line: `image: prom/prometheus:v2.52.0`)
- Grafana: `docker-compose.observability.yml` (line: `image: grafana/grafana:10.4.2`)

## Service Roles

- **Jaeger 1.59**: Distributed tracing collector and UI. Receives traces from gateway service via OTLP/gRPC on port 4317
- **Prometheus v2.52.0**: Metrics scraper and time-series database. Scrapes gateway metrics on port 8081/metrics
- **Grafana 10.4.2**: Metrics visualization. Connects to Prometheus datasource, displays dashboards

## Safe Upgrade Workflow

### 1. Plan Upgrade Order
Upgrade observability services in this order:
1. Prometheus (rarely breaks scrape endpoints)
2. Grafana (breaks if dashboard JSON incompatible)
3. Jaeger (most likely to break trace format)

### 2. Upgrade Single Service
```bash
# Edit docker-compose.observability.yml
# Change: image: grafana/grafana:10.4.2
# To:     image: grafana/grafana:10.5.0

# Build and test just that service
docker compose -f docker-compose.observability.yml build grafana
docker compose -f docker-compose.observability.yml up grafana
```

### 3. Check Service Health

#### Prometheus
```bash
# Should return 200
curl http://localhost:9090/-/healthy
curl http://localhost:9090/api/v1/targets
```

#### Grafana
```bash
# Should return 200
curl http://localhost:3000/api/health
curl http://localhost:3000/api/datasources
```

#### Jaeger
```bash
# Should return JSON with services
curl http://localhost:16686/api/services
```

### 4. Validate Data Flow

```bash
# 1. Generate some traffic to gateway
curl http://localhost:8080/api/meme/analyze?image_url=https://example.com/meme.jpg

# 2. Check traces appear in Jaeger UI
# Open http://localhost:16686
# Select gateway service, view traces

# 3. Check metrics in Prometheus
# Open http://localhost:9090
# Search for: gateway_requests_total

# 4. Check dashboards in Grafana
# Open http://localhost:3000 (admin/admin)
# Verify all panels showing data
```

### 5. Commit Changes
```bash
git add docker-compose.observability.yml
git commit -m "upgrade(observability): update prometheus and grafana

- Prometheus v2.52.0 → v2.53.0 (performance improvements)
- Grafana 10.4.2 → 10.5.0 (bug fixes, UI improvements)

Testing: All services healthy, traces in Jaeger, metrics in Prometheus"
```

## Service-Specific Upgrade Guides

### Prometheus Upgrades

**Compatibility**: Very stable, most versions work with any recent Jaeger/Grafana

**Pre-upgrade**:
```bash
# Check current targets
curl http://localhost:9090/api/v1/targets | jq '.data.activeTargets'
```

**Upgrade**:
```bash
# Edit docker-compose.observability.yml
sed -i '' 's/prom\/prometheus:v2.52.0/prom\/prometheus:v2.53.0/' docker-compose.observability.yml

# Rebuild and restart
docker compose -f docker-compose.observability.yml up --build prometheus
```

**Post-upgrade**:
```bash
# Wait for startup (30-60 seconds)
sleep 30

# Verify targets are healthy
curl http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | {labels: .labels, health: .health}'
```

**Known Issues**:
- Storage format may require migration (happens automatically)
- Very large databases might take minutes to start
- Config changes (scrape intervals) require restart

### Grafana Upgrades

**Compatibility**: Generally compatible with same Prometheus; dashboards might need tweaks

**Pre-upgrade**:
```bash
# Backup dashboards (exported to JSON)
# Or access Grafana at http://localhost:3000 and export manually
```

**Upgrade**:
```bash
# Edit docker-compose.observability.yml
sed -i '' 's/grafana\/grafana:10.4.2/grafana\/grafana:10.5.0/' docker-compose.observability.yml

docker compose -f docker-compose.observability.yml up --build grafana
```

**Post-upgrade**:
```bash
# Login: http://localhost:3000 (admin/admin)
# Check: Settings > Datasources > Prometheus (should be green)
# Check: All dashboards load without errors
# If dashboards broken: re-import or edit panel JSON
```

**Breaking Changes to Watch**:
- Dashboard JSON structure (sometimes needs panel re-creation)
- Datasource API changes (rare in minor versions)
- Plugin compatibility (if using plugins)

### Jaeger Upgrades

**Compatibility**: All-in-one mode is fairly stable, but trace API may change

**Pre-upgrade** (optional backup):
```bash
# If Jaeger has persistent storage, consider backing up
# Default all-in-one uses in-memory storage (no persistence needed)
```

**Upgrade**:
```bash
# Edit docker-compose.yml
sed -i '' 's/jaegertracing\/all-in-one:1.59/jaegertracing\/all-in-one:1.60/' docker-compose.yml

docker compose up --build jaeger
```

**Post-upgrade**:
```bash
# Generate a trace
curl http://localhost:8080/health

# Check UI: http://localhost:16686
# Select service "gateway", should see recent traces
```

**Breaking Changes to Watch**:
- OTLP endpoint format (usually stable)
- Trace storage format (all-in-one mode handles this)
- UI changes (rare to affect programmatic access)

## Full Stack Upgrade

To safely upgrade all three services at once:

```bash
# 1. Edit both compose files
# docker-compose.yml: Jaeger version
# docker-compose.observability.yml: Prometheus and Grafana versions

# 2. Test individual services first
docker compose build jaeger prometheus grafana

# 3. Bring up observability stack
docker compose -f docker-compose.observability.yml up --build

# 4. Verify all services healthy
curl http://localhost:16686/api/services
curl http://localhost:9090/-/healthy
curl http://localhost:3000/api/health

# 5. Verify full integration
docker compose up --build gateway ai_service
curl http://localhost:8080/health
# Check Jaeger, Prometheus, Grafana all seeing data
```

## Pre-Upgrade Checklist

- [ ] Note current versions before upgrading
- [ ] Have docker/docker-compose working
- [ ] Understand which services depend on each other
- [ ] Plan upgrade order (Prometheus → Grafana → Jaeger)
- [ ] Know how to access each service's health endpoint

## Post-Upgrade Validation

- [ ] All containers start without errors: `docker compose logs`
- [ ] Prometheus targets are healthy: http://localhost:9090/api/v1/targets
- [ ] Grafana datasources are connected: http://localhost:3000/api/datasources
- [ ] Jaeger services appear: http://localhost:16686/api/services
- [ ] Gateway can start and export traces/metrics
- [ ] Sample request generates trace and metrics
- [ ] Dashboards display data in Grafana

## Rollback if Issues

```bash
# Revert compose files
git checkout docker-compose.yml docker-compose.observability.yml

# Rebuild with old versions
docker compose down
docker compose -f docker-compose.observability.yml down
docker compose --build
```

## Monitoring Strategy During Upgrades

While upgrading, monitor for:
1. **Jaeger**: Trace completeness (all spans present)
2. **Prometheus**: Scrape success rate (targets healthy)
3. **Grafana**: Dashboard panel rendering (no broken panels)

If any service fails to upgrade:
1. Check logs: `docker compose logs [service-name]`
2. Revert the specific service version
3. Research incompatibility on GitHub/Docker Hub
4. Try intermediate version before jumping multiple versions

## Resource Limits

If services start slower after upgrade, they may need more resources:
```yaml
# In docker-compose.observability.yml
services:
  prometheus:
    deploy:
      resources:
        limits:
          cpus: '1'
          memory: 1G
```
