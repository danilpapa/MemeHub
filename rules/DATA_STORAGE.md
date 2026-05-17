# MemeHub: Слои хранения данных

Документ описывает добавленные слои хранения данных: что реализовано, зачем и как использовать.

## Зачем это нужно

До реализации в проекте не было постоянного хранилища:

- job'ы хранились in-memory в gateway и **терялись при каждом рестарте**
- изображения передавались как base64 прямо в Kafka-событиях (~670 KB на файл 500 KB)
- результаты анализа нигде не сохранялись долгосрочно

---

## Архитектура хранилищ

```
клиент
  │
  ▼
gateway ──── upload image ────► MinIO (холодное хранилище)
  │              └─ presigned URL ─► Kafka event (вместо base64)
  │
  ├── Redis ◄──────────────────────── job status (queued/completed/failed)
  ├── PostgreSQL ◄─────────────────── job results (долгосрочно)
  │
  ▼
Kafka ──► ai_service ──► анализ изображения
                │
                └── ClickHouse ◄──── аналитика (теги, эмоции, время обработки)
```

---

## Слой 1 — Redis (горячее хранилище)

**Заменил:** `Arc<RwLock<HashMap<String, JobRecord>>>` в памяти gateway.

**Что даёт:**
- job'ы переживают рестарт gateway
- TTL 24 часа — автоматическая очистка старых записей
- быстрый доступ O(1) для `GET /ai/jobs/:job_id`

**Файлы:**
- `gateway/src/services/redis_store.rs` — сервис `RedisJobStore`

**Конфиг:**
```env
REDIS_URL=redis://redis:6379
```

**Docker:**
```
redis:7-alpine   port 6379   volume: redis_data
```

---

## Слой 2 — MinIO (холодное / объектное хранилище)

**Что даёт:**
- изображения хранятся в S3-совместимом хранилище, а не в Kafka
- Kafka-сообщения уменьшились с ~670 KB до ~200 байт (URL вместо base64)
- изображения можно посмотреть через MinIO Console

**Как работает:**
1. `POST /ai/process` принимает файл
2. Gateway загружает файл в MinIO бакет `memes` по ключу `uploads/{job_id}/{filename}`
3. Gateway генерирует presigned GET URL (TTL 1 час)
4. В Kafka-событии уходит `ImageSource::Url { image_url: presigned_url }` вместо base64
5. ai_service скачивает по URL через httpx (код не менялся)

> Если `MINIO_ENDPOINT` не задан — fallback на старое поведение (base64 в Kafka).

**Файлы:**
- `gateway/src/services/minio.rs` — сервис `MinioService`

**Конфиг:**
```env
MINIO_ENDPOINT=http://minio:9000
MINIO_ACCESS_KEY=minioadmin
MINIO_SECRET_KEY=minioadmin
MINIO_BUCKET=memes
```

**Docker:**
```
minio/minio:latest   port 9000 (API)   port 9001 (Console)   volume: minio_data
minio/mc:latest      minio-init — создаёт бакет memes при старте
```

MinIO Console: http://localhost:9001 (minioadmin / minioadmin)

---

## Слой 3 — PostgreSQL (реляционное хранилище)

**Что даёт:**
- долгосрочное хранение результатов анализа, не зависящее от TTL Redis
- SQL-запросы к истории job'ов и результатов

**Схема:**

```sql
-- статусы и метаданные задач
CREATE TABLE meme_jobs (
    job_id     TEXT        PRIMARY KEY,
    status     TEXT        NOT NULL,           -- queued | completed | failed
    image_ref  TEXT,                           -- URL изображения в MinIO
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- результаты анализа
CREATE TABLE meme_results (
    job_id       TEXT        PRIMARY KEY REFERENCES meme_jobs (job_id),
    ocr_text     TEXT        NOT NULL,
    tags         TEXT[]      NOT NULL,
    emotion      TEXT        NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Миграции запускаются автоматически при старте gateway.

> Если `DATABASE_URL` не задан — PostgreSQL отключается, gateway работает без него.

**Файлы:**
- `gateway/migrations/0001_initial.sql` — SQL-схема
- `gateway/src/services/db.rs` — сервис `Database`

**Конфиг:**
```env
DATABASE_URL=postgres://memehub:memehub@postgres:5432/memehub?sslmode=disable
```

**Docker:**
```
postgres:16-alpine   port 5432   volume: pg_data
POSTGRES_DB=memehub  POSTGRES_USER=memehub  POSTGRES_PASSWORD=memehub
```

Подключиться вручную:
```bash
docker compose exec postgres psql -U memehub -d memehub
```

---

## Слой 4 — ClickHouse (аналитика)

**Что даёт:**
- аналитические запросы по тегам, эмоциям, throughput
- не нагружает PostgreSQL аналитикой

**Таблица `meme_events`:**

| Поле | Тип | Описание |
|---|---|---|
| `job_id` | String | ID задачи |
| `tags` | Array(String) | теги из анализа |
| `emotion` | String | эмоция |
| `ocr_length` | UInt32 | длина OCR-текста |
| `processing_ms` | UInt32 | время обработки мс |
| `created_at` | DateTime | время записи |

Таблица создаётся автоматически при первом старте ai_service.

**Файлы:**
- `ai_service/main.py` — функции `_init_clickhouse()`, `_send_analytics()`

**Конфиг:**
```env
CLICKHOUSE_HOST=clickhouse
CLICKHOUSE_PORT=8123
```

**Docker:**
```
clickhouse/clickhouse-server:24.5   port 8123 (HTTP API)   port 19000 (native)   volume: clickhouse_data
```

HTTP UI: http://localhost:8123/play

Пример запроса:
```sql
SELECT emotion, count() AS cnt
FROM meme_events
GROUP BY emotion
ORDER BY cnt DESC
```

---

## Grafana

Provisioning datasources настроен автоматически при запуске observability-стека:

- **PostgreSQL** — подключён к `postgres:5432 / memehub`
- **ClickHouse** — подключён через плагин `grafana-clickhouse-datasource`

Файл: `observability/grafana/datasources/datasource.yaml`

Запуск с observability:
```bash
docker compose -f docker-compose.yml -f docker-compose.observability.yml up --build -d
```

Grafana: http://localhost:3001

---

## Запуск полного стека

```bash
docker compose up --build -d
```

### Проверка сервисов

```bash
# Redis
docker compose exec redis redis-cli ping
# → PONG

# PostgreSQL
docker compose exec postgres psql -U memehub -d memehub -c '\dt'
# → meme_jobs, meme_results

# MinIO
curl http://localhost:9000/minio/health/live
# → 200 OK

# ClickHouse
curl http://localhost:8123/ping
# → Ok.

# Отправить мем
curl -X POST http://localhost:8080/ai/process \
  -F "file=@meme.png"
# → {"job_id":"...","status":"queued"}

# Проверить job (из Redis)
curl http://localhost:8080/ai/jobs/<job_id>

# ClickHouse аналитика
curl 'http://localhost:8123/?query=SELECT+*+FROM+meme_events+LIMIT+5+FORMAT+JSONEachRow'
```

---

## Изменённые файлы

| Файл | Что изменилось |
|---|---|
| `docker-compose.yml` | добавлены Redis, PostgreSQL, MinIO+init, ClickHouse |
| `gateway/Cargo.toml` | `redis 0.25`, `aws-sdk-s3 1`, `sqlx 0.8` |
| `gateway/Dockerfile` | копирует `migrations/` для compile-time embed |
| `gateway/migrations/0001_initial.sql` | новый — SQL-схема |
| `gateway/src/services/redis_store.rs` | новый — RedisJobStore |
| `gateway/src/services/minio.rs` | новый — MinioService |
| `gateway/src/services/db.rs` | новый — Database |
| `gateway/src/services/mod.rs` | pub mod для новых сервисов |
| `gateway/src/config/config.rs` | новые env-переменные |
| `gateway/src/Models/AppState.rs` | redis, minio, db вместо jobs |
| `gateway/src/Models/jobs.rs` | добавлен Deserialize |
| `gateway/src/handlers/jobs.rs` | Redis store + MinIO upload |
| `gateway/src/services/kafka.rs` | consumer пишет в Redis + DB |
| `gateway/src/main.rs` | инициализация новых сервисов |
| `ai_service/main.py` | ClickHouse аналитика через httpx |
| `docker-compose.observability.yml` | Grafana provisioning + плагин |
| `observability/grafana/datasources/datasource.yaml` | новый — datasources |
