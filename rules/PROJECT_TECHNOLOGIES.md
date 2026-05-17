# MemeHub: технологии и архитектурные решения

Документ описывает текущую техническую карту проекта: из каких сервисов он состоит, какие технологии используются, как устроены основные решения и на что обращать внимание при развитии.

## Кратко о проекте

MemeHub сейчас реализован как контейнеризированная микросервисная система:

- `gateway` - HTTP gateway на Rust, принимает внешние запросы и проксирует AI-запросы во внутренний сервис.
- `ai_service` - Python/FastAPI сервис обработки мемов: OCR, простая классификация тегов/эмоции и опциональная LLM-классификация.
- `redpanda` - Kafka-compatible брокер для event-driven связи между gateway и AI service.
- `observability` - Jaeger для distributed tracing, Prometheus для метрик, Grafana для визуализации.
- `docker-compose.yml` и `docker-compose.observability.yml` - локальная оркестрация сервисов.
- `fastlane/Fastfile`, `Makefile`, `scripts/` - вспомогательные команды для CI/dev workflow.
- `rules/` - правила безопасного обновления зависимостей и инфраструктуры.

## Архитектура сервисов

Основной поток запроса:

1. Клиент отправляет запрос на `gateway` по адресу `http://localhost:8080`.
2. Gateway обрабатывает middleware, request id, tracing и metrics.
3. Запрос `POST /ai/process` превращается в job и публикуется в Kafka topic `meme.analysis.requested`.
4. `ai_service` читает событие из Kafka, принимает изображение как base64-файл или ссылку `image_url`.
5. Сервис извлекает текст через OCR и публикует результат в Kafka topic `meme.analysis.completed`.
6. Gateway читает completed event, обновляет in-memory job store и отдает результат через `GET /ai/jobs/{job_id}`.
7. Для обратной совместимости остальные запросы вида `/ai/*path` могут проксироваться в `ai_service`.
8. Результат анализа содержит:
   - `ocr_text`
   - `tags`
   - `emotion`

В Docker Compose связка выглядит так:

- `gateway` слушает порт `8080`.
- `ai_service` слушает порт `8000`.
- `redpanda` слушает Kafka broker внутри сети на `9092`, а с хоста доступен на `19092`.
- `jaeger` слушает UI на `16686` и OTLP/gRPC на `4317`.
- `prometheus` слушает `9090` при подключении observability compose-файла.
- `grafana` слушает `3001` с пробросом на внутренний `3000`.

## Rust Gateway

Расположение: `gateway/`

### Основные технологии

- Rust edition `2024`.
- `axum 0.7` - HTTP server, routing, extractors, middleware.
- `tokio 1` с `features = ["full"]` - async runtime.
- `reqwest 0.12` - HTTP-клиент для проксирования запросов в AI service.
- `tower-http 0.5` - request id и trace middleware.
- `tracing`, `tracing-subscriber`, `tracing-appender` - структурированные JSON-логи.
- `opentelemetry 0.27`, `opentelemetry-otlp 0.27`, `tracing-opentelemetry 0.28` - экспорт trace span'ов в Jaeger через OTLP.
- `metrics 0.23`, `metrics-exporter-prometheus 0.15` - сбор и экспорт Prometheus-метрик.
- `rdkafka 0.36` - Kafka producer/consumer для EDA flow.
- `serde`, `serde_json` - JSON event contracts.
- `uuid` - генерация `job_id`.
- `base64`, `multer` - упаковка multipart upload в Kafka event.

### Структура gateway

- `gateway/src/main.rs` - точка входа: читает конфиг, поднимает logging/tracing/metrics, создает `ProxyService`, запускает Axum server.
- `gateway/src/config/config.rs` - конфигурация через переменные окружения.
- `gateway/src/app/app.rs` - сборка `Router`, middleware и routes.
- `gateway/src/services/proxy.rs` - основная логика проксирования запросов.
- `gateway/src/services/kafka.rs` - Kafka producer и consumer completed events.
- `gateway/src/handlers/proxy.rs` - handler для `/ai/*path`.
- `gateway/src/handlers/jobs.rs` - постановка analysis job и чтение job status.
- `gateway/src/handlers/metrics.rs` - handler для `/metrics`.
- `gateway/src/middleware/metrics.rs` - middleware для измерения длительности HTTP-запросов.
- `gateway/src/observability/logging.rs` - JSON-логи + OpenTelemetry tracing.
- `gateway/src/observability/metrics.rs` - Prometheus recorder.
- `gateway/src/Models/AppState.rs` - общий state приложения.

### Gateway routes

- `GET /metrics` - Prometheus exposition endpoint.
- `POST /ai/process` - EDA endpoint: ставит задачу анализа в Kafka и возвращает `202 Accepted`.
- `GET /ai/jobs/:job_id` - возвращает статус и результат фоновой задачи.
- `ANY /ai/*path` - прокси в AI service.

Важно: отдельного `GET /health` в текущем коде gateway нет, хотя в некоторых документах он упоминается как проверочный endpoint.

### Конфигурация gateway

Переменные окружения:

- `AI_SERVICE_URL` - base URL AI-сервиса. В Docker: `http://ai_service:8000`.
- `OTEL_EXPORTER_OTLP_ENDPOINT` - OTLP endpoint. В Docker: `http://jaeger:4317`.
- `OTEL_SERVICE_NAME` - имя сервиса в трассировке. По умолчанию `gateway`.
- `RUST_LOG` - уровень логирования, в Dockerfile установлен `info`.
- `KAFKA_BOOTSTRAP_SERVERS` - Kafka broker list. В Docker: `redpanda:9092`, локально с хоста: `localhost:19092`.
- `KAFKA_ANALYSIS_REQUESTED_TOPIC` - topic для задач анализа, по умолчанию `meme.analysis.requested`.
- `KAFKA_ANALYSIS_COMPLETED_TOPIC` - topic для результатов анализа, по умолчанию `meme.analysis.completed`.

Fallback для `AI_SERVICE_URL` в коде сейчас равен `http://localhost:3000`, хотя локальный AI service по умолчанию слушает `8000`. Для запуска вне Docker лучше явно задавать `AI_SERVICE_URL=http://localhost:8000`.

### Решение для проксирования

`ProxyService`:

- принимает исходный Axum `Request<Body>`;
- сохраняет HTTP method;
- копирует заголовки, кроме `Host`;
- сохраняет query string;
- читает body в bytes;
- отправляет запрос через `reqwest::Client`;
- возвращает upstream status, headers и body клиенту;
- оборачивает upstream call в span `proxy_upstream`.

Это простое reverse-proxy решение без streaming body. Для больших файлов или потоковой загрузки можно будет заменить чтение всего тела через `to_bytes` на streaming-подход.

### Решение для Kafka / EDA

Gateway реализует job-based API:

1. `POST /ai/process` принимает multipart `file` или JSON `{ "image_url": "..." }`.
2. Gateway генерирует `job_id`.
3. В in-memory хранилище создается запись со статусом `queued`.
4. Gateway публикует JSON-событие `AnalysisRequestedEvent` в topic `meme.analysis.requested`.
5. Background consumer gateway слушает topic `meme.analysis.completed`.
6. Когда приходит `AnalysisCompletedEvent`, gateway обновляет job record.
7. Клиент забирает результат через `GET /ai/jobs/{job_id}`.

Event contracts:

```json
{
  "job_id": "uuid",
  "image": {
    "type": "file",
    "filename": "meme.png",
    "content_type": "image/png",
    "data_base64": "..."
  }
}
```

или:

```json
{
  "job_id": "uuid",
  "image": {
    "type": "url",
    "image_url": "https://example.com/meme.png"
  }
}
```

Completed event:

```json
{
  "job_id": "uuid",
  "status": "completed",
  "result": {
    "ocr_text": "...",
    "tags": ["meme"],
    "emotion": "neutral"
  },
  "error": null
}
```

### Observability в gateway

Gateway использует три слоя наблюдаемости:

- Request tracing через `TraceLayer::new_for_http()`.
- Request ID через header `x-request-id`.
- Prometheus histogram `http_request_duration_seconds`.

Логи пишутся в `logs/gateway.jsonl` в JSONL-формате. В Docker эта папка проброшена как `./gateway/logs:/app/logs`.

Trace span'ы:

- `http_request` - общий span на входящий HTTP-запрос.
- `proxy_upstream` - span на вызов AI service.

## Python AI Service

Расположение: `ai_service/`

### Основные технологии

- Python `3.11` в Docker.
- `FastAPI 0.111.0` - HTTP API.
- `uvicorn 0.30.1` - ASGI server.
- `pydantic 2.7.4` - request/response модели.
- `Pillow 10.3.0` - загрузка и нормализация изображений.
- `pytesseract 0.3.10` + system package `tesseract-ocr` - OCR.
- `httpx 0.27.0` - скачивание изображения по URL.
- `torch 2.3.1`, `transformers 4.41.2`, `accelerate 0.31.0`, `safetensors 0.4.3` - опциональный LLM-пайплайн.
- `aiokafka 0.10.0` - Kafka consumer/producer.

### AI service routes

- `GET /health` - health check, возвращает `{"status": "ok"}`.
- `POST /process` - обработка изображения.

`POST /process` поддерживает два режима входа:

- multipart upload через `file`;
- JSON body с `image_url`.

Этот endpoint остался синхронным и полезен для прямой диагностики AI service. В обычном EDA-flow gateway вызывает AI service не через HTTP, а через Kafka events.

Ответ:

```json
{
  "ocr_text": "text from image",
  "tags": ["meme"],
  "emotion": "neutral"
}
```

### Решение для OCR и тегирования

Пайплайн обработки:

1. Изображение загружается из bytes или скачивается по URL.
2. `Pillow` открывает изображение и приводит его к `RGB`.
3. `pytesseract.image_to_string` извлекает текст.
4. Если `USE_LLM=0`, применяется простая эвристика:
   - `lol`, `lmao`, `аха`, `ахах`, `хаха` -> `humor`, `joy`;
   - `sad`, `печаль`, `грусть` -> `sad`, `sadness`;
   - `angry`, `злюсь`, `бесит` -> `angry`, `anger`;
   - иначе `meme`, `neutral`.
5. Если `USE_LLM=1`, загружается Hugging Face model pipeline и модель пытается вернуть JSON с тегами и эмоцией.
6. Если LLM не смогла обработать текст или вернуть корректный JSON, сервис падает обратно на простую эвристику.

### LLM-режим

Переменные окружения:

- `USE_LLM=1` - включает LLM-классификацию.
- `QWEN_MODEL` - имя модели, по умолчанию `Qwen/Qwen2.5-0.5B-Instruct`.
- `PORT` - порт сервиса, по умолчанию `8000`.
- `KAFKA_BOOTSTRAP_SERVERS` - Kafka broker list.
- `KAFKA_ANALYSIS_REQUESTED_TOPIC` - topic входящих задач.
- `KAFKA_ANALYSIS_COMPLETED_TOPIC` - topic результатов.

Модель кэшируется в глобальной переменной `_LLM_PIPE`, чтобы не загружать ее на каждый запрос.

Device selection:

- `mps`, если доступен Apple Silicon backend;
- иначе `cpu`.

### Kafka consumer в AI service

При старте FastAPI создает background task:

- consumer group: `ai-service-analysis`;
- input topic: `meme.analysis.requested`;
- output topic: `meme.analysis.completed`.

Consumer читает событие, восстанавливает изображение из base64 или скачивает по URL, запускает тот же OCR/tagging pipeline, что и `/process`, и публикует completed/failed event.

## Docker и запуск

### Основной стек

`docker-compose.yml` поднимает:

- `redpanda`
- `ai_service`
- `gateway`
- `jaeger`

Команда:

```bash
docker compose up --build -d
```

### Полный стек с observability

`docker-compose.observability.yml` добавляет:

- `prometheus`
- `grafana`

Команда:

```bash
docker compose -f docker-compose.yml -f docker-compose.observability.yml up --build -d
```

### Dockerfile gateway

Gateway собирается multi-stage:

- build stage: `rust:1.88-slim`;
- runtime stage: `debian:bookworm-slim`;
- дополнительно ставятся `build-essential`, `cmake`, `pkg-config`, `libcurl4-openssl-dev`, `libssl-dev` для сборки `rdkafka` и `libssl3` для runtime.

### Dockerfile ai_service

AI service использует:

- `python:3.11-slim`;
- system dependency `tesseract-ocr`;
- `pip install -r requirements.txt`;
- запуск через `python main.py`.

## Prometheus, Jaeger, Grafana

## Kafka / Redpanda

Redpanda используется как локальный Kafka-compatible broker без ZooKeeper.

Compose settings:

- internal broker: `redpanda:9092`;
- host broker: `localhost:19092`;
- admin/API port: `9644`;
- image: `redpandadata/redpanda:v24.2.7`.

Topics:

- `meme.analysis.requested` - задачи анализа от gateway к AI service.
- `meme.analysis.completed` - результаты анализа от AI service к gateway.

Проверить topics:

```bash
docker compose exec redpanda rpk topic list
```

Посмотреть сообщения можно через `rpk topic consume`, например:

```bash
docker compose exec redpanda rpk topic consume meme.analysis.completed -n 1
```

### Jaeger

Jaeger используется в `all-in-one` режиме:

- image: `jaegertracing/all-in-one:1.59`;
- `COLLECTOR_OTLP_ENABLED=true`;
- OTLP/gRPC endpoint: `4317`;
- UI: `http://localhost:16686`.

Gateway отправляет trace spans через OpenTelemetry OTLP exporter.

### Prometheus

Prometheus config: `observability/prometheus.yml`.

Текущая scrape config:

```yaml
scrape_configs:
  - job_name: "gateway"
    metrics_path: /metrics
    static_configs:
      - targets: ["gateway:8080"]
```

Prometheus UI доступен на `http://localhost:9090`.

### Grafana

Grafana:

- image: `grafana/grafana:10.4.2`;
- внешний порт: `3001`;
- volume: `grafana_data`.

Datasource и dashboards в репозитории пока не зафиксированы, поэтому их нужно настраивать вручную или добавить provisioning-файлы в будущем.

## Данные

### Текущее состояние

Сейчас в проекте нет постоянного хранилища:

- job'ы хранятся in-memory в gateway и теряются при рестарте;
- Kafka topic'и служат временным буфером событий (retention по умолчанию);
- изображения передаются как base64 прямо в Kafka-событии, без отдельного хранилища.

### Горячее хранилище — Redis

Redis заменит in-memory job store в gateway:

- хранение статусов job'ов с TTL (переживает рестарт процесса);
- кэш результатов анализа (TTL ~24h, чтобы не перезапускать OCR при повторных запросах);
- опционально: pub/sub для push-уведомлений о завершении job'а вместо polling.

Compose:

- image: `redis:7-alpine`;
- port: `6379`;
- volume: `redis_data` для RDB/AOF persistence.

В gateway: заменить `HashMap<Uuid, Job>` в `AppState` на обращение к Redis через `redis` crate.

### Реляционное хранилище — PostgreSQL

PostgreSQL для долгосрочного хранения результатов:

- таблица `meme_jobs`: `job_id`, `status`, `created_at`, `completed_at`, `image_ref`;
- таблица `meme_results`: `job_id`, `ocr_text`, `tags[]`, `emotion`, `model_version`;
- первичный ключ `job_id` — UUID, совпадает с gateway job_id.

Compose:

- image: `postgres:16-alpine`;
- port: `5432`;
- volume: `pg_data`.

Миграции: `sqlx-migrate` (Rust/gateway) или Alembic (Python/ai_service).

### Холодное хранилище — MinIO

MinIO как S3-compatible object storage для изображений:

- бакет `memes` — оригиналы загруженных изображений;
- бакет `memes-processed` — нормализованные RGB-версии после Pillow;
- gateway при получении файла загружает его в MinIO, в Kafka-событие кладет `image_ref` (ключ объекта) вместо base64;
- ai_service скачивает изображение из MinIO по ключу.

Это устраняет текущее ограничение: base64 большого файла раздувает Kafka-сообщение.

Compose:

- image: `minio/minio:latest`;
- port API: `9000`;
- port Console: `9001`;
- volume: `minio_data`;
- env: `MINIO_ROOT_USER`, `MINIO_ROOT_PASSWORD`.

Переменные окружения для сервисов:

- `MINIO_ENDPOINT` — например, `http://minio:9000`;
- `MINIO_ACCESS_KEY`, `MINIO_SECRET_KEY`;
- `MINIO_BUCKET_MEMES=memes`.

### Аналитика — ClickHouse

ClickHouse для аналитических запросов по результатам анализа:

- таблица `meme_events`: `job_id`, `tags`, `emotion`, `ocr_length`, `processing_ms`, `created_at`;
- ai_service или отдельный consumer пишет в ClickHouse после завершения анализа;
- запросы: топ тегов, распределение эмоций, throughput по времени, средняя длина OCR-текста.

Compose:

- image: `clickhouse/clickhouse-server:24`;
- port HTTP: `8123`;
- port native: `9000` (конфликт с MinIO — назначить `19000`);
- volume: `clickhouse_data`.

### Grafana Dashboards для данных

Предлагаемые дашборды поверх ClickHouse и Prometheus:

- **Meme Analysis Overview**: количество job'ов в сутки, успешные vs failed, средняя задержка обработки.
- **Tags & Emotions**: bar chart топ-10 тегов, pie chart по эмоциям за выбранный период.
- **Pipeline Throughput**: запросы в минуту через gateway, lag в Kafka consumer group.
- **Storage**: размер бакетов MinIO, объём PostgreSQL.

Datasources для добавления в Grafana provisioning:

- Prometheus — уже собирается gateway;
- ClickHouse — через плагин `grafana-clickhouse-datasource`;
- PostgreSQL — встроенный datasource Grafana.

Provisioning-файлы рекомендуется зафиксировать в `observability/grafana/datasources/` и `observability/grafana/dashboards/`.

### Рекомендуемый порядок внедрения

1. Redis — минимальная замена in-memory store, наименьший риск.
2. MinIO + обновить event contract (убрать base64 из Kafka).
3. PostgreSQL + миграции.
4. ClickHouse + Grafana dashboards.

## Makefile и scripts

### Makefile targets

- `make onboarding` - делает `scripts/setup.sh` executable и запускает setup.
- `make update` - ставит `servicectl` через Cargo из GitHub.
- `make container` - запускает `servicectl`.
- `make scratch` - открывает Docker Desktop и запускает `docker compose up --build`.

### scripts/setup.sh

Скрипт для macOS-oriented onboarding:

- устанавливает Homebrew, если его нет;
- устанавливает Rust через `rustup-init`;
- добавляет Cargo в shell profile;
- устанавливает `servicectl`.

### scripts/open_docker.sh

Открывает Docker Desktop через:

```bash
open -a "Docker"
```

и ждет, пока `docker info` начнет отвечать.

## CI/CD и Fastlane

`fastlane/Fastfile` содержит lane:

```ruby
lane :build_ci do
  sh("cd .. && cargo check --manifest-path gateway/Cargo.toml")
  sh("cd .. && docker compose build ai_service gateway")
end
```

То есть CI-валидация проекта сейчас завязана на:

- `cargo check` для gateway;
- Docker build для `ai_service` и `gateway`.

В `rules/cicd-tooling-upgrades.md` описан ожидаемый GitHub Actions workflow, но файла `.github/workflows/build-on-main.yml` в текущем дереве проекта нет.

## Правила обновления зависимостей

Папка `rules/` фиксирует upgrade-политику проекта. Главные принципы:

- обновлять один компонент за раз;
- отдельно обновлять CI/tooling, gateway, AI service и observability;
- перед обновлениями проверять changelog и compatibility;
- после обновлений запускать локальные проверки и Docker build;
- фиксировать breaking changes в commit message;
- держать `Cargo.toml` и `Cargo.lock` согласованными.

Критичные связки:

- OpenTelemetry crates в gateway должны обновляться согласованно.
- `axum` и `tower-http` должны быть совместимы.
- `torch`, `transformers`, `accelerate` должны проверяться вместе.
- Python major version нужно проверять на совместимость с PyTorch.

Рекомендуемый порядок обновления:

1. CI/CD и tooling.
2. Docker/base images.
3. Python AI service.
4. Rust gateway.
5. Observability stack.

## Важные замечания и несостыковки

Текущий проект рабоче описывает направление, но есть несколько мест, которые стоит держать в голове:

- В gateway нет route `/health`, хотя документация и upgrade rules местами предлагают проверять `http://localhost:8080/health`.
- В AI service есть `/health`, но документация местами упоминает `/ping`.
- В документации observability местами встречается порт `8081/metrics`, но в коде gateway и Prometheus config используется `8080/metrics`.
- Fallback `AI_SERVICE_URL` в gateway равен `http://localhost:3000`, хотя AI service по умолчанию запускается на `8000`.
- В `rules/` упоминается `.github/workflows/build-on-main.yml`, но такого файла в репозитории сейчас нет.
- Автоматических тестов в текущем дереве проекта не найдено.
- Proxy читает request body целиком в память; для больших изображений или high-load сценариев это стоит пересмотреть.
- LLM-пайплайн загружается лениво, но без явного ограничения размера модели/памяти. В Docker по умолчанию `USE_LLM=0`, что защищает локальный запуск от тяжелой загрузки модели.

## Рекомендованные следующие улучшения

- Добавить `GET /health` в gateway, чтобы документация и runtime совпадали.
- Исправить fallback `AI_SERVICE_URL` на `http://localhost:8000`.
- Унифицировать документацию: `/health` вместо `/ping`, `8080/metrics` вместо `8081/metrics`.
- Добавить минимальные тесты:
  - Rust: проверка router/routes и proxy behavior через mock upstream.
  - Python: тесты `/health`, `/process` с mock OCR/image.
- Добавить GitHub Actions workflow, если CI реально нужен.
- Добавить Grafana provisioning для datasource/dashboards.
- Ограничить размер body в gateway или перейти на streaming proxy для больших файлов.

## Быстрые команды

Основной запуск:

```bash
docker compose up --build -d
```

Полный запуск с observability:

```bash
docker compose -f docker-compose.yml -f docker-compose.observability.yml up --build -d
```

Проверка AI service:

```bash
curl http://localhost:8000/health
```

Поставить EDA-задачу анализа:

```bash
curl -X POST http://localhost:8080/ai/process \
  -F "file=@$HOME/Downloads/meme.png"
```

Проверить job:

```bash
curl http://localhost:8080/ai/jobs/<job_id>
```

Прямая синхронная проверка AI service:

```bash
curl -X POST http://localhost:8000/process \
  -F "file=@$HOME/Downloads/meme.png"
```

Проверка gateway metrics:

```bash
curl http://localhost:8080/metrics
```

Проверка Rust gateway:

```bash
cargo check --manifest-path gateway/Cargo.toml
```

CI lane:

```bash
fastlane build_ci
```
