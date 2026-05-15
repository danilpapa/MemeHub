# Как работать в проекте

### Перед запуском
запуск скрипта 
```bash
make onboarding
```
она установит: homebre, rustc, servicectl

### Добавление мироксервиса
сервисы добавлять в файл docker-compose.yml

### Точки входа

| Сервис | URL | Описание |
|--------|-----|---------|
| **Nginx** (основной) | `http://localhost:80` | Rate-limited entry point |
| Gateway (прямой) | `http://localhost:8080` | Только для отладки |
| Grafana | `http://localhost:3001` | Дашборды (auto-provisioned) |
| Prometheus | `http://localhost:9090` | Метрики |
| Jaeger UI | `http://localhost:16686` | Distributed traces |
| MinIO Console | `http://localhost:9001` | Хранилище изображений |

### Сборка проекта
- Только приложение + Jaeger:
```bash
docker compose up --build -d
```
- Приложение + Jaeger + Prometheus + Grafana + дашборды:
```bash
docker compose -f docker-compose.yml -f docker-compose.observability.yml up --build -d
```

### Rate Limiting

Nginx ограничивает 10 запросов/сек с хоста, burst до 20. При превышении отвечает `429 Too Many Requests`.

Для тестирования rate limit:
```bash
# Быстро отправить 25 запросов
for i in $(seq 1 25); do curl -s -o /dev/null -w "%{http_code}\n" http://localhost/ai/jobs/test; done
```

### Масштабирование gateway

Nginx настроен как load balancer. Для запуска нескольких реплик:
```bash
docker compose up --build -d --scale gateway=3
```

### Grafana дашборды

Дашборды provisioned автоматически из `observability/grafana/dashboards/`.

После запуска полного стека открой `http://localhost:3001` (admin/admin) → папка **MemeHub**.

Дашборд **MemeHub Gateway**:
- Request rate по маршрутам
- Error rate (5xx %)
- Latency P50/P95/P99
- Requests by status code

### Как проверить trace в UI:
1. Подними стек командой выше.
2. Отправь запрос:
```bash
curl -i -X POST http://localhost:8080/ai/process
```
или любой реальный запрос в твой gateway.
3. Открой Jaeger UI:
[http://localhost:16686](http://localhost:16686)
4. В поле `Service` выбери `gateway`
5. Нажми `Find Traces`

Если всё ок, увидишь trace со span’ами `http_request` и `proxy_upstream`.

Если пересобрался только `gateway` то:
- Пересобрать и перезапустить только gateway:
```bash
docker compose up --build -d gateway
```
- Если observability стек уже поднят, для полного проекта обычно удобнее:
```bash
docker compose -f docker-compose.yml -f docker-compose.observability.yml up --build -d gateway
```
- Если код не менялся, а нужен просто рестарт:
```bash
docker compose restart gateway
```

- Логи gateway:
```bash
docker compose logs -f gateway
```
- Логи AI service:
```bash
docker compose logs -f ai_service
```
- Логи Redpanda/Kafka:
```bash
docker compose logs -f redpanda
```
- Логи Jaeger:
```bash
docker compose logs -f jaeger
```
- Остановить всё:
```bash
docker compose down
```
- Остановить всё вместе с observability stack:
```bash
docker compose -f docker-compose.yml -f docker-compose.observability.yml down
```

### Kafka / EDA flow

Сейчас связь `gateway -> ai_service` для анализа мема работает через Kafka-compatible брокер Redpanda.

Запрос через gateway больше не возвращает результат сразу. Он ставит задачу в очередь и возвращает `job_id`:

```bash
curl -X POST http://localhost:8080/ai/process \
  -F "file=@$HOME/Downloads/meme.png"
```

Ответ:

```json
{
  "job_id": "uuid",
  "status": "queued"
}
```

Проверить результат:

```bash
curl http://localhost:8080/ai/jobs/<job_id>
```

Когда обработка завершится, ответ будет содержать `status: "completed"` и поле `result`.

Прямая синхронная проверка AI service осталась доступна:

```bash
curl -X POST http://localhost:8000/process \
  -F "file=@$HOME/Downloads/meme.png"
```

Посмотреть Kafka topics:

```bash
docker compose exec redpanda rpk topic list
```

Основные topics:

- `meme.analysis.requested` - gateway публикует задачи анализа.
- `meme.analysis.completed` - AI service публикует результат обработки.
