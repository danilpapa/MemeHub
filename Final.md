# MemeHub: Полная архитектура инфраструктуры
---

## Часть 1: Основная архитектура MemeHub (Docker Compose)

### Общая схема

```
Клиент
  ↓
[Gateway Service] (Rust/Axum, порт 8080)
  ├─ HTTP: /ai/process, /ai/jobs/{id}
  ├─ /metrics для Prometheus
  ├─ OpenTelemetry traces → Jaeger
  │
  ├─→ Kafka (Redpanda) — event-driven архитектура
  │     ├─ Topic: meme.analysis.requested
  │     └─ Topic: meme.analysis.completed
  │
  ├─→ Redis (горячее хранилище)
  │     └─ job status (queued/completed/failed, TTL 24h)
  │
  ├─→ PostgreSQL (холодное хранилище)
  │     ├─ meme_jobs (метаданные)
  │     └─ meme_results (результаты анализа)
  │
  └─→ MinIO (объектное хранилище S3-compatible)
        └─ исходные изображения, presigned URLs в Kafka
  
[AI Service] (Python/FastAPI, порт 8000)
  ├─ Kafka consumer: слушает meme.analysis.requested
  ├─ OCR + простая классификация (или LLM если USE_LLM=1)
  ├─ Kafka producer: публикует meme.analysis.completed
  │
  └─→ ClickHouse
        └─ аналитика: теги, эмоции, время обработки

[Observability Stack]
  ├─ Jaeger (порт 16686) — distributed tracing
  ├─ Prometheus (порт 9090) — metrics scraping
  └─ Grafana (порт 3001) — dashboards и visualization
```

### Ключевой паттерн: Event-Driven Architecture

**Синхронный API** (для простых случаев):
```
POST /ai/process (с файлом)
  ↓
Gateway генерирует job_id и загружает файл в MinIO
  ↓
Возвращает 202 Accepted + job_id
```

**Асинхронная обработка** (основной flow):
```
1. Gateway публикует AnalysisRequestedEvent в Kafka
   {
     "job_id": "uuid",
     "image": { "type": "url", "image_url": "presigned MinIO URL" }
   }

2. AI Service читает из Kafka (consumer group: ai-service-analysis)
   - Скачивает изображение с MinIO
   - Запускает pytesseract для OCR
   - Классифицирует эмоции (heuristic или LLM)
   - Пишет результаты в ClickHouse

3. AI Service публикует AnalysisCompletedEvent в Kafka
   {
     "job_id": "uuid",
     "status": "completed",
     "result": { "ocr_text": "...", "tags": [...], "emotion": "..." }
   }

4. Gateway читает completed event через background consumer
   - Обновляет Redis статус с job_id
   - Сохраняет результат в PostgreSQL

5. Клиент пулит GET /ai/jobs/{job_id}
   - Gateway возвращает статус и результат из Redis/PostgreSQL
```

### Почему именно такая архитектура?

- **Kafka вместо прямого HTTP**: Асинхронный, decoupled, resilient к сбоям AI service
- **Redis вместо in-memory**: Job'ы переживают рестарт gateway, быстрый доступ
- **PostgreSQL для долгосрочного хранения**: SQL-запросы, история, аудит
- **MinIO вместо base64 в Kafka**: Kafka-сообщения lean (~200 байт вместо 670 KB), изображения можно просматривать
- **ClickHouse для аналитики**: OLAP-оптимизирован для queries по тегам/эмоциям, не нагружает PostgreSQL

---

## Часть 2: Слои хранения данных

### Слой 1: Redis (горячее, in-memory)

```
Назначение: текущие job status и очень свежий кэш
TTL: 24 часа
Структура: { job_id → JobRecord { status, result, created_at } }

Файл: gateway/src/services/redis_store.rs
Конфиг: REDIS_URL=redis://redis:6379
```

**Проблема**: Без Redis job'ы терялись при перезагрузке gateway
**Решение**: Добавили Redis service, gateway теперь обращается к нему в AppState

### Слой 2: PostgreSQL (реляционное, долгосрочное)

```
Таблица meme_jobs:
  job_id (TEXT, PRIMARY KEY)
  status (TEXT) — queued | completed | failed
  image_ref (TEXT) — path в MinIO
  created_at (TIMESTAMPTZ)

Таблица meme_results:
  job_id (TEXT, PRIMARY KEY, FK → meme_jobs)
  ocr_text (TEXT)
  tags (TEXT[]) — массив
  emotion (TEXT)
  completed_at (TIMESTAMPTZ)

Файл: gateway/migrations/0001_initial.sql
Конфиг: DATABASE_URL=postgres://memehub:memehub@postgres:5432/memehub
```

**Проблема**: Результаты анализа нигде не сохранялись
**Решение**: Kafka consumer в gateway теперь обновляет PostgreSQL на каждый completed event

### Слой 3: MinIO (объектное хранилище)

```
Назначение: изображения (оригиналы, обработанные)
S3-compatible API
Бакет: memes
Структура: uploads/{job_id}/{filename}

Файл: gateway/src/services/minio.rs
Конфиг:
  MINIO_ENDPOINT=http://minio:9000
  MINIO_ACCESS_KEY=minioadmin
  MINIO_SECRET_KEY=minioadmin
  MINIO_BUCKET=memes
```

**Flow**:
1. `POST /ai/process` с файлом
2. Gateway загружает файл в MinIO → получает object key
3. Генерирует presigned GET URL (TTL 1 час)
4. Публикует в Kafka только URL (вместо base64)
5. AI Service скачивает с URL и обрабатывает

**Проблема**: base64-файл размером 500KB раздувал Kafka-сообщение, затруднял обработку
**Решение**: MinIO как объектное хранилище, в Kafka только URLs

### Слой 4: ClickHouse (аналитика)

```
Таблица meme_events:
  job_id (String)
  tags (Array(String))
  emotion (String)
  ocr_length (UInt32) — длина текста
  processing_ms (UInt32) — время обработки
  created_at (DateTime)

Файл: ai_service/main.py функция _send_analytics()
Конфиг:
  CLICKHOUSE_HOST=clickhouse
  CLICKHOUSE_PORT=8123
```

**Назначение**: Аналитические queries без нагрузки на PostgreSQL
**Примеры**:
```sql
-- Топ 10 эмоций за день
SELECT emotion, COUNT() as cnt FROM meme_events 
WHERE created_at > now() - INTERVAL 1 DAY
GROUP BY emotion ORDER BY cnt DESC LIMIT 10

-- Средняя скорость обработки
SELECT avg(processing_ms) FROM meme_events
```

---

## Часть 3: Observability (видимость в систему)

### Слой 1: Distributed Tracing (Jaeger)

```
Назначение: отследить path одного request'а через сервисы
Ports:
  4317 — OTLP/gRPC endpoint (куда gateway отправляет spans)
  16686 — UI

Gateway генерирует spans:
  ├─ http_request — входящий HTTP запрос с headers
  ├─ proxy_upstream — если есть прямой proxy в AI service
  └─ kafka_produce — публикация события в Kafka

Конфиг gateway:
  OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
  OTEL_SERVICE_NAME=gateway

Файл: gateway/src/observability/logging.rs
```

**UI**: http://localhost:16686
- Select service "gateway"
- View timeline спана, latency, дети спанов
- Поиск по job_id через tags

### Слой 2: Metrics (Prometheus)

```
Назначение: числовые метрики для дашбордов и alerting
Endpoint: http://gateway:8080/metrics

Gateway публикует:
  - http_request_duration_seconds (histogram с buckets)
  - kafka_messages_published_total (counter)
  - redis_operations_duration_seconds (histogram)
  - postgres_queries_duration_seconds (histogram)

Prometheus scrapes:
  - job_name: "gateway"
    targets: ["gateway:8080"]
    scrape_interval: 15s
```

**Проблема**: Без метрик нельзя построить дашборды и понять bottleneck'и
**Решение**: metrics-exporter-prometheus в gateway, tower-http middleware

### Слой 3: Dashboards (Grafana)

```
Ports:
  3001 — UI (login: admin / admin)

Provisioning (автоматический setup):
  datasources:
    - Prometheus: http://prometheus:9090
    - PostgreSQL: postgresql://memehub:memehub@postgres:5432/memehub
    - ClickHouse: plugin grafana-clickhouse-datasource

Файл: observability/grafana/datasources/datasource.yaml
```

**Предлагаемые дашборды**:
1. **Pipeline Overview**: Requests per minute, success rate, avg latency
2. **Analysis Metrics**: Tags distribution (bar chart), emotions (pie chart)
3. **Storage**: Redis memory, PostgreSQL size, MinIO buckets
4. **System**: Gateway CPU/Memory, AI service resource usage

### Слой 4: Logging (JSON logs)

```
Gateway логирует в файл в JSONL-формате:
  logs/gateway.jsonl

Каждая строка — JSON с fields:
  {
    "timestamp": "2026-05-17T10:30:45.123Z",
    "level": "INFO",
    "message": "job completed",
    "job_id": "uuid",
    "ocr_length": 150,
    "emotion": "humor",
    "trace_id": "...",
    "span_id": "..."
  }

В Docker: volume ./gateway/logs:/app/logs
```

---

## Часть 4: Kubernetes & Terraform (для защиты/экзамена)

### Проблема: Docker Compose ≠ Production

Docker Compose удобен для local development, но:
- Нет resource limits (один контейнер съест всю машину)
- Нет автоматического restart/scaling
- Нет load balancing между репликами
- Нет rolling updates
- Нет network policies и RBAC

**Решение**: Kubernetes для production, Terraform для Infrastructure-as-Code

### Архитектура в Kubernetes

```
                         ┌──────────────────┐
                         │   Nginx Ingress  │ (LoadBalancer)
                         │ (ports 80/443)   │
                         └────────┬─────────┘
                                  │
                    ┌─────────────┼─────────────┐
                    ▼             ▼             ▼
                Gateway-1    Gateway-2    Gateway-3
                  (Replica 1)  (Replica 2) (Replica 3)
                    │             │             │
                    └─────────────┼─────────────┘
                                  │
              ┌───────────────────┼───────────────────┐
              ▼                   ▼                   ▼
          Redis              PostgreSQL           MinIO
         (Service)           (Service)          (Service)
              │                   │                   │
              │                   │                   │
         redis:6379         postgres:5432       minio:9000
```

### Namespaces (изоляция ресурсов)

```hcl
resource "kubernetes_namespace" "memehub" {
  metadata {
    name = "memehub"
    labels = { "app.kubernetes.io/name" = "memehub" }
  }
}

resource "kubernetes_namespace" "ingress_nginx" {
  metadata {
    name = "ingress-nginx"
  }
}
```

**Зачем**: Разные namespace для разных целей — чистота, RBAC, resource quotas

### Service Accounts (pod identity)

```hcl
resource "kubernetes_service_account" "gateway" {
  metadata {
    name      = "gateway"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
}

resource "kubernetes_service_account" "ai_service" {
  metadata {
    name      = "ai-service"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
}

resource "kubernetes_service_account" "nginx_ingress" {
  metadata {
    name      = "nginx-ingress"
    namespace = kubernetes_namespace.ingress_nginx.metadata[0].name
  }
}
```

**Зачем**: Каждому поду свой identity для RBAC (что может делать)

### RBAC: Role + RoleBinding

```hcl
# Nginx может читать конфиг из ConfigMap
resource "kubernetes_role" "nginx_ingress" {
  metadata {
    name      = "nginx-ingress"
    namespace = kubernetes_namespace.ingress_nginx.metadata[0].name
  }
  rule {
    api_groups = [""]
    resources  = ["configmaps"]
    verbs      = ["get", "list", "watch"]
  }
}

# Привязываем Role к ServiceAccount
resource "kubernetes_role_binding" "nginx_ingress" {
  metadata {
    name      = "nginx-ingress"
    namespace = kubernetes_namespace.ingress_nginx.metadata[0].name
  }
  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "Role"
    name      = kubernetes_role.nginx_ingress.metadata[0].name
  }
  subject {
    kind      = "ServiceAccount"
    name      = kubernetes_service_account.nginx_ingress.metadata[0].name
    namespace = kubernetes_namespace.ingress_nginx.metadata[0].name
  }
}
```

**Принцип**: least privilege — дай только то, что нужно

### Secrets (зашифрованные credentials)

```hcl
resource "kubernetes_secret" "postgres" {
  metadata {
    name      = "postgres-credentials"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
  type = "Opaque"
  data = {
    username = base64encode("memehub")
    password = base64encode(var.postgres_password) # из terraform.tfvars
    database = base64encode("memehub")
  }
}

resource "kubernetes_secret" "minio" {
  metadata {
    name      = "minio-credentials"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
  type = "Opaque"
  data = {
    access_key = base64encode(var.minio_access_key)
    secret_key = base64encode(var.minio_secret_key)
  }
}
```

**Зачем**: Кредиты безопасно, не в git (terraform.tfvars в .gitignore)

### Ingress Controller (маршрутизация)

```yaml
# k8s/10-ingress-controller.yaml

apiVersion: apps/v1
kind: Deployment
metadata:
  name: nginx-ingress
  namespace: ingress-nginx
spec:
  replicas: 1
  template:
    spec:
      serviceAccountName: nginx-ingress
      containers:
      - name: nginx
        image: nginx:latest
        ports:
        - containerPort: 80
        - containerPort: 443
        volumeMounts:
        - name: config
          mountPath: /etc/nginx/nginx.conf
          subPath: nginx.conf
      volumes:
      - name: config
        configMap:
          name: nginx-config

---
apiVersion: v1
kind: ConfigMap
metadata:
  name: nginx-config
  namespace: ingress-nginx
data:
  nginx.conf: |
    upstream gateway {
      server gateway.memehub.svc.cluster.local:8080;
    }
    server {
      listen 80;
      location / {
        proxy_pass http://gateway;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
      }
    }
    
    # Rate limiting
    limit_req_zone $binary_remote_addr zone=api_limit:10m rate=10r/s;
    server {
      location / {
        limit_req zone=api_limit burst=20;
        proxy_pass http://gateway;
      }
    }

---
apiVersion: v1
kind: Service
metadata:
  name: ingress
  namespace: ingress-nginx
spec:
  type: LoadBalancer
  ports:
  - port: 80
    targetPort: 80
  - port: 443
    targetPort: 443
  selector:
    app: nginx-ingress
```

### Terraform Workflow

```bash
# 1. Инициализация — скачивает provider
terraform init

# 2. План — показывает что будет создано
terraform plan

# 3. Применение — создает ресурсы
terraform apply

# 4. Проверка
kubectl get namespaces
kubectl get serviceaccounts -n memehub
kubectl get secrets -n memehub
```

---

## Часть 5: Проблемы и решения (история внедрения)

### Проблема 1: Nginx pod не стартовал (ErrImagePull)

```
Error: Failed to pull image 'nginx:latest': short read: expected 10229 bytes but got 0
```

**Причина**: Network issue при скачивании образа из docker.io
**Решение**: 
```bash
kubectl delete pod nginx-ingress-xyz -n ingress-nginx
# Kubernetes автоматически пересоздает pod, retry-механизм срабатывает
```
**Результат**: Pod успешно запустился со second attempt

### Проблема 2: Terraform not found

```
zsh: command not found: terraform
```

**Причина**: Terraform не в PATH (Homebrew установка не удалась)
**Решение**:
```bash
brew tap hashicorp/tap
brew install hashicorp/tap/terraform
terraform version
```

### Проблема 3: Namespace conflict

```
Error: namespaces 'memehub' already exists
```

**Причина**: Namespaces созданы через `kubectl apply` ранее, но Terraform state не знает о них
**Решение**: Импортировать existing ресурсы в Terraform state
```bash
terraform import kubernetes_namespace.memehub memehub
terraform import kubernetes_namespace.ingress_nginx ingress-nginx
terraform import kubernetes_service_account.nginx_ingress ingress-nginx/nginx-ingress
```
**Результат**: Terraform теперь управляет существующими ресурсами

### Проблема 4: Terraform outputs не показываются

```
No outputs found — The state file either has no outputs defined
```

**Причина**: outputs.tf создан, но state не обновлен
**Решение**:
```bash
terraform apply -auto-approve
```
**Результат**: State обновился, outputs успешно отображаются

### Проблема 5: Rate limiting не реализован в Docker Compose

**Статус**: В текущей версии Docker Compose нет rate limiting на уровне Gateway
**Где находится**: Реализован только на уровне Nginx Ingress в Kubernetes
**Конфиг**:
```
limit_req_zone $binary_remote_addr zone=api_limit:10m rate=10r/s;
```

---

## Часть 6: Почему все максимально компактно

### Выборы архитектуры

| Компонент | Выбор | Почему |
|-----------|-------|--------|
| Маршрутизация | Nginx Ingress (K8s) | Lightweight, встроенная rate limiting через ConfigMap |
| Async messaging | Kafka/Redpanda | Event-driven, decoupled, resilient |
| Hot storage | Redis | O(1) access, TTL auto-cleanup |
| Cold storage | PostgreSQL | SQL queries, durability, ACID |
| Objects | MinIO | S3-compatible, lean Kafka messages |
| Analytics | ClickHouse | OLAP-оптимизирован, не нагружает DB |
| Tracing | Jaeger | All-in-one, OTLP standard |
| Metrics | Prometheus | Стандарт, Grafana support |

### Компактность в Terraform

- **Минимальные RBAC**: Только необходимые rules для каждого сервиса
- **Никакого clustering**: Single replica для development (добавить легко)
- **Никакого TLS/SSL**: Ingress на HTTP, TLS можно добавить через cert-manager позже
- **Единый state file**: Локальный state.tfstate (в production → S3/Terraform Cloud)
- **.gitignore**: Исключаем *.tfstate, terraform.tfvars, .terraform/

### Без чего обошлись

- ❌ Helm charts (можно добавить для production)
- ❌ ArgoCD (для GitOps deployment)
- ❌ Service Mesh (Istio, Linkerd)
- ❌ Network Policies (пока не нужны)
- ❌ PersistentVolumes (используем managed storages)
- ❌ StatefulSets (Stateless services)

---

## Часть 7: Итоговая архитектура

### Слои системы

```
┌─────────────────────────────────────────────────────────┐
│ Layer 1: Ingress & Load Balancing                      │
│ ├─ Nginx Ingress Controller                            │
│ ├─ Rate Limiting (10 req/s, burst 20)                  │
│ └─ Health checks, SSL termination                      │
├─────────────────────────────────────────────────────────┤
│ Layer 2: Application Services (Kubernetes Deployments) │
│ ├─ Gateway Service (Rust/Axum)                         │
│ │  ├─ HTTP API: /ai/process, /ai/jobs/{id}, /metrics  │
│ │  ├─ OpenTelemetry → Jaeger                           │
│ │  └─ Prometheus metrics                               │
│ │                                                       │
│ └─ AI Service (Python/FastAPI)                         │
│    ├─ OCR processing (pytesseract)                     │
│    ├─ Emotion classification                           │
│    └─ ClickHouse analytics sink                        │
├─────────────────────────────────────────────────────────┤
│ Layer 3: Event Bus                                     │
│ └─ Kafka/Redpanda Topics:                              │
│    ├─ meme.analysis.requested                          │
│    └─ meme.analysis.completed                          │
├─────────────────────────────────────────────────────────┤
│ Layer 4: Data Storage                                  │
│ ├─ Redis (Горячее, TTL 24h)                            │
│ ├─ PostgreSQL (Холодное, ACID)                         │
│ ├─ MinIO (Объекты, S3-compatible)                      │
│ └─ ClickHouse (Аналитика, OLAP)                        │
├─────────────────────────────────────────────────────────┤
│ Layer 5: Observability                                 │
│ ├─ Jaeger (Distributed Tracing)                        │
│ ├─ Prometheus (Metrics)                                │
│ ├─ Grafana (Dashboards)                                │
│ └─ JSON Logs (JSONL files)                             │
└─────────────────────────────────────────────────────────┘
```

### Критические цепочки

1. **Request path**: Client → Nginx Ingress → Gateway Service → Redis + PostgreSQL + Kafka
2. **Tracing path**: Application → OpenTelemetry SDK → Jaeger OTLP/gRPC → Jaeger UI
3. **Metrics path**: Application → Prometheus client lib → Prometheus scraper → Grafana
4. **Async processing**: Kafka publisher (Gateway) → Topic → Kafka consumer (AI Service)

---

## Часть 8: Checklist реализации

- ✅ Docker Compose микросервисная архитектура (Gateway + AI Service)
- ✅ Event-driven messaging через Kafka/Redpanda
- ✅ Четыре слоя хранения (Redis, PostgreSQL, MinIO, ClickHouse)
- ✅ Полная observability (Jaeger, Prometheus, Grafana, JSON logs)
- ✅ Kubernetes deployment manifests (Namespaces, ServiceAccounts, RBAC, Secrets)
- ✅ Terraform Infrastructure-as-Code (main.tf, namespaces.tf, service_accounts.tf, secrets.tf)
- ✅ Nginx Ingress Controller (маршрутизация + rate limiting)
- ✅ Все проблемы решены (ErrImagePull, namespace conflicts, output visibility)

### Что можно добавить позже (future)

- Service Mesh для advanced traffic management
- Helm charts для standardization
- ArgoCD для GitOps CD pipeline
- PDB (Pod Disruption Budgets) для high availability
- Horizontal Pod Autoscaler (HPA) для auto-scaling
- TLS certificates через cert-manager
- Network Policies для segmentation

---

### Observability URLs
- **Jaeger**: http://localhost:16686
- **Prometheus**: http://localhost:9090
- **Grafana**: http://localhost:3001 (admin/admin)
- **MinIO Console**: http://localhost:9001 (minioadmin/minioadmin)
- **ClickHouse Play**: http://localhost:8123/play
