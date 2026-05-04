# CI/CD & Tooling Upgrades

Managing GitHub Actions workflows, Fastlane, and development tooling versions.

**Locations**:
- `.github/workflows/build-on-main.yml` — CI/CD pipeline
- `fastlane/Fastfile` — Build automation (Fastlane 2.228.0)
- `Makefile` — Development commands
- `scripts/setup.sh` — Installation script

## Components to Upgrade

### 1. Fastlane

**Current version**: 2.228.0  
**Location**: `.github/workflows/build-on-main.yml` (Ruby gem)  
**Purpose**: Automates builds for iOS/Android and custom validation

#### Check Current Version
```bash
# Local
fastlane --version

# In CI (view GitHub Actions logs)
# Workflow installs via: gem install fastlane
```

#### Safe Upgrade Workflow

1. **Check for updates**:
   ```bash
   gem list fastlane
   gem search fastlane
   ```

2. **Update locally first**:
   ```bash
   gem install fastlane -v 2.229.0
   fastlane --version  # Verify
   ```

3. **Test locally**:
   ```bash
   fastlane build_ci  # Run the same command CI uses
   ```

4. **Update CI/CD workflow** (`.github/workflows/build-on-main.yml`):
   ```yaml
   - name: Install Fastlane
     run: gem install fastlane -v 2.229.0
   ```

5. **Commit and test**:
   ```bash
   git add .github/workflows/build-on-main.yml
   git commit -m "upgrade(ci): fastlane 2.228.0 → 2.229.0"
   git push
   # Watch GitHub Actions run with new version
   ```

**Breaking changes**: Rare, but check release notes on GitHub: fastlane/fastlane/releases

### 2. Ruby Version

**Current version**: 3.3 (implicit in GitHub Actions)  
**Location**: `.github/workflows/build-on-main.yml`  
**Purpose**: Runtime for Fastlane

#### Check and Upgrade

```yaml
# In .github/workflows/build-on-main.yml

- uses: ruby/setup-ruby@v1
  with:
    ruby-version: '3.3'  # Change if needed
    bundler-cache: true  # Caches gems
```

**Safe versions**: 3.2, 3.3, 3.4 (as of May 2026)

**Upgrade**:
1. Update `ruby-version: '3.3'` → `ruby-version: '3.4'`
2. Test locally: `ruby --version` should match
3. Push and verify CI passes
4. Commit with note about Ruby version

### 3. GitHub Actions Versions

**Check current actions** in `.github/workflows/build-on-main.yml`:

```yaml
- uses: actions/checkout@v4        # Latest: v4
- uses: dtolnay/rust-toolchain@v1  # Rust toolchain setup
- uses: ruby/setup-ruby@v1         # Ruby setup
```

#### Upgrade Steps

1. **Review action updates**:
   ```bash
   # Check GitHub action release notes
   # e.g., actions/checkout@v3 → actions/checkout@v4
   ```

2. **Update action versions**:
   ```yaml
   - uses: actions/checkout@v4
   - uses: dtolnay/rust-toolchain@v1
   ```

3. **Test**: Push and let GitHub Actions run

**Safe approach**: Update one action at a time and verify CI still passes

### 4. Rust Toolchain

**Location**: `.github/workflows/build-on-main.yml`  
**Current**: Latest stable (via `dtolnay/rust-toolchain@v1`)

```yaml
- uses: dtolnay/rust-toolchain@v1
  with:
    toolchain: stable  # Latest stable compiler
```

**Options**:
- `stable` — Latest stable (recommended, what we use)
- `1.80` — Specific version pinning (if you need reproducibility)

**Rarely needs updating**: The action automatically fetches latest stable.

### 5. Makefile Targets

**Location**: `/Makefile`  
**Purpose**: Development convenience commands

```makefile
.PHONY: onboarding container scratch clean

onboarding:
	bash scripts/setup.sh
	make container

container:
	servicectl build gateway ai_service --no-cache

scratch:
	docker compose down -v

clean:
	rm -rf gateway/target
	rm -rf ai_service/__pycache__
```

**No version management needed**, but review if Docker Compose commands change:
- If Docker Compose v2 syntax changes, update commands here
- Currently uses: `docker compose` (v2, not `docker-compose`)

### 6. Setup Scripts

**Location**: `scripts/setup.sh`  
**Purpose**: Installs dependencies (Homebrew, Rust, servicectl)

```bash
#!/bin/bash

# Installs:
# - Homebrew (macOS package manager)
# - Rust toolchain
# - servicectl (custom service manager)
```

#### Upgrade Considerations

If this script installs external tools, consider version pinning:

```bash
# Example: If script installs specific versions
brew install python@3.12

# Or pins Rust version
rustup install stable
```

**Review script** if Rust Edition changes (currently 2024):
```bash
# If upgrading to Rust 2025 Edition
rustup update stable
```

## Common Upgrade Scenarios

### Scenario A: Fastlane Minor Update (2.228 → 2.229)
✅ Safe, usually just bug fixes
```bash
# Local test
gem install fastlane -v 2.229.0
fastlane build_ci

# Update CI workflow
sed -i '' 's/2.228.0/2.229.0/' .github/workflows/build-on-main.yml

git add .github/workflows/build-on-main.yml
git commit -m "upgrade(ci): fastlane 2.228.0 → 2.229.0"
```

### Scenario B: Ruby Major Update (3.3 → 3.4)
⚠️ Test locally first, but usually safe
```bash
# Update local Ruby (using rbenv or similar)
ruby --version

# Test Fastlane still works
fastlane build_ci

# Update CI workflow
sed -i '' "s/ruby-version: '3.3'/ruby-version: '3.4'/" .github/workflows/build-on-main.yml

# Push and verify CI passes with new Ruby
```

### Scenario C: GitHub Actions Update (checkout@v3 → v4)
✅ Very safe, backward compatible
```bash
# Update action version
sed -i '' 's/@v3/@v4/' .github/workflows/build-on-main.yml

git add .github/workflows/build-on-main.yml
git commit -m "upgrade(ci): actions/checkout v3 → v4"
```

### Scenario D: Rust Toolchain Edition (2021 → 2024)
🔴 Requires code changes if using new Edition features
```bash
# Gateway already uses 2024 Edition
# But if upgrading: edit gateway/Cargo.toml
# edition = "2024"

# Then run: cargo fix --edition --allow-dirty
# And test thoroughly
```

## Pre-Upgrade Checklist

- [ ] Current CI/CD workflow is passing (green checks)
- [ ] Local environment matches CI/CD (same Ruby, Rust, tooling)
- [ ] Understand which versions CI/CD uses
- [ ] Review changelogs for breaking changes
- [ ] Plan upgrade order (least to most critical)

## Post-Upgrade Validation

### Local Testing
```bash
# Test Fastlane command that CI runs
fastlane build_ci

# Check no new warnings
fastlane build_ci 2>&1 | grep -i "warn"
```

### CI/CD Validation
After pushing upgrade:
1. GitHub Actions runs automatically on push
2. Check workflow results: Actions tab
3. Verify all checks pass (✓)
4. Review logs for any warnings or deprecations

### Rollback if Issues

```bash
# Revert workflow file
git checkout .github/workflows/build-on-main.yml

# Force push if already pushed to remote
git push --force-with-lease

# Or create a revert commit
git revert HEAD
git push
```

## CI/CD Workflow Structure

Current workflow triggers on:
- **Pull Request** to main
- **Push** to main

```yaml
on:
  push:
    branches: [main]
  pull_request:
    branches: [main]
```

**Upgrade impact**: Be careful not to break trigger conditions when editing

## Local Development Environment Parity

Ensure your local setup matches CI/CD:

```bash
# Check Ruby version
ruby --version  # Should be 3.3+

# Check Rust version
rustc --version
cargo --version  # Should be stable

# Check Fastlane version
fastlane --version  # Should match CI/CD

# Check Docker
docker --version
docker compose version
```

**If mismatch**: Either update locally or update CI/CD workflow

## Sensitive Changes During CI/CD Upgrades

⚠️ Avoid these during same PR:
- Major Rust version bumps (use separate PR)
- Dependency version changes (use separate PR)
- Build command changes (use separate PR)

**Why**: Makes debugging CI failures much harder. Upgrade tooling separately, then upgrade dependencies.

## Documentation References

- **Fastlane**: https://docs.fastlane.tools/
- **GitHub Actions**: https://docs.github.com/en/actions
- **Rust Toolchain**: https://www.rust-lang.org/tools/install
- **Ruby**: https://www.ruby-lang.org/

## Testing CI/CD Changes Locally

Before pushing CI/CD changes, test on your machine:

```bash
# Simulate CI environment
bash scripts/setup.sh
make onboarding
make container

# Run the exact Fastlane lane CI uses
fastlane build_ci

# If it passes locally, it's likely to pass in CI
```

If test fails locally, fix before pushing to GitHub.
