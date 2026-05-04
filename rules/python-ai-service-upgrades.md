# Python AI Service Upgrades

Managing requirements.txt dependencies for the FastAPI-based AI service.

**Location**: `/ai_service/requirements.txt`  
**Current Key Dependencies**:
- FastAPI 0.111.0 (web framework)
- PyTorch 2.3.1 (deep learning)
- Transformers 4.41.2 (NLP models)
- Accelerate 0.31.0 (GPU/distributed training)

## Safe Upgrade Workflow

### 1. Create Virtual Environment for Testing
```bash
cd ai_service
python3 -m venv test_env
source test_env/bin/activate
pip install -r requirements.txt
```

### 2. Check for Updates
```bash
# See available updates
pip list --outdated

# Create an updated requirements.txt (for review)
pip install pip-tools
pip-compile requirements.txt --upgrade --dry-run
```

### 3. Review Breaking Changes
Check changelogs for:
- **FastAPI**: API changes, dependency changes
- **PyTorch**: Model serialization format changes, API changes
- **Transformers**: Model loading changes, tokenizer changes, cache format
- **Accelerate**: Multi-GPU setup changes

### 4. Update Dependencies Incrementally
Start with smaller updates, then move to major ones:

```bash
# Option A: Update one dependency at a time
pip install --upgrade fastapi==0.112.0

# Option B: Update a category (safer for AI/ML)
pip install --upgrade "transformers>=4.42.0,<5.0.0"

# Option C: Update all (riskier)
pip install --upgrade -r requirements.txt
```

### 5. Freeze and Test
```bash
# Freeze your working environment to requirements.txt
pip freeze > requirements.txt

# Test AI service locally
python main.py

# Or with Docker
docker compose build ai_service
docker compose up ai_service

# Test an API endpoint
curl http://localhost:8000/ping
```

### 6. Validate Integration
```bash
# Test gateway can reach AI service
docker compose up --build
curl http://localhost:8080/api/meme/analyze?image_url=...
```

### 7. Commit Changes
```bash
git add ai_service/requirements.txt
git commit -m "upgrade(ai_service): update fastapi and transformers

- FastAPI 0.111.0 → 0.112.0 (bug fixes)
- Transformers 4.41.2 → 4.42.0 (model loading improvements)
- PyTorch 2.3.1 → 2.4.0 (performance improvements)

Testing: pip install verified, local test_env works,
docker compose up ai_service succeeds, API endpoints respond"
```

## Critical Dependencies & Compatibility

### ML Framework Tier (Careful Coordination Required)
- `torch` (PyTorch) 2.3.1
- `transformers` 4.41.2
- `accelerate` 0.31.0

**Upgrade rule**: When updating PyTorch, always verify Transformers compatibility. Check:
```
PyTorch 2.3 → 2.4: Check Transformers compatibility
Transformers 4.41 → 4.42: Usually compatible with recent PyTorch
```

### Web Framework Tier
- `fastapi` 0.111.0
- `uvicorn` 0.30.1 (ASGI server, usually auto-updated with FastAPI)
- `pydantic` 2.7.4 (data validation)

**Upgrade rule**: Pydantic major versions (2.x → 3.x) have breaking changes. Update carefully.

### Utility & Processing Tier
- `pillow` 10.3.0 (image processing)
- `pytesseract` 0.3.10 (OCR wrapper)
- `httpx` 0.27.0 (async HTTP client)

**Upgrade rule**: These are fairly stable. Safe to update frequently.

## Common Upgrade Scenarios

### Scenario A: FastAPI Minor Update (0.111 → 0.112)
✅ Safe, usually backward compatible
```bash
pip install --upgrade fastapi==0.112.0
pip freeze > requirements.txt
python main.py  # Quick test
```

### Scenario B: PyTorch Major Update (2.3 → 2.4)
⚠️ Potential breaking changes, requires full testing
1. Check PyTorch release notes for API changes
2. Test with: `pip install torch==2.4.0`
3. Run full AI service with sample inputs
4. Verify model loading still works
5. Check GPU memory usage (if applicable)

### Scenario C: Transformers Major Update (4.41 → 5.0)
🔴 Major version, likely breaking changes
1. Carefully review Transformers changelog
2. Check for tokenizer API changes
3. Verify model caching still works
4. Test end-to-end: `docker compose up --build`
5. Validate meme analysis outputs match expectations

## Pre-Upgrade Checklist

- [ ] Current main branch is clean (`git status` is clean)
- [ ] Virtual environment is active
- [ ] Current requirements.txt installs without errors
- [ ] AI service runs locally without errors
- [ ] Review dependency changelogs on pypi.org
- [ ] Check for known compatibility issues between dependencies

## Post-Upgrade Validation

- [ ] `pip install -r requirements.txt` succeeds
- [ ] `python ai_service/main.py` starts without errors
- [ ] API endpoint `/ping` responds with 200
- [ ] Test meme analysis with a sample image (not in USE_LLM=0 mode)
- [ ] Docker image builds: `docker compose build ai_service`
- [ ] Integration test: gateway can call AI service
- [ ] No deprecation warnings in logs

## Machine Learning Specific Checks

When upgrading ML dependencies:

1. **Model Format Compatibility**
   - Ensure old saved models can load with new library versions
   - Check if model cache directory structure changed

2. **Tensor Operations**
   - If custom code exists, verify tensor operations still work
   - Check for deprecated PyTorch ops

3. **GPU Compatibility** (if using GPU)
   - Verify CUDA version compatibility with new PyTorch
   - Check Accelerate multi-GPU setup if used

4. **Performance Regressions**
   - Monitor inference time after upgrades
   - Compare model outputs on same test inputs

## Dependency Group Strategy

Upgrade in this order (safer to riskier):
1. **Web Framework**: FastAPI, Uvicorn, Pydantic (safest)
2. **Utilities**: Pillow, Pytesseract, HTTPX (safe)
3. **ML Framework**: Transformers, Accelerate (careful)
4. **Core ML**: PyTorch (most careful, test thoroughly)

## Rollback if Issues

```bash
# Revert requirements.txt to last known good
git checkout ai_service/requirements.txt

# Reinstall known-good dependencies
pip install -r ai_service/requirements.txt

# Verify service works
python ai_service/main.py
```

## Environment Variables for Testing

```bash
# Mock mode (no actual model loading)
export USE_LLM=0
python ai_service/main.py

# Real mode (loads actual models, slower startup)
export USE_LLM=1
python ai_service/main.py
```
