# MemeHub Kubernetes Deployment

Все файлы здесь описывают развёртывание MemeHub на локальном Kubernetes кластере с Cilium CNI.

## Быстрый старт

### Автоматизированная установка (Рекомендуется)

```bash
./setup-k3s-cilium.sh
```

Этот скрипт:
1. Устанавливает k3s
2. Устанавливает Helm
3. Устанавливает Cilium
4. Создаёт namespace `memehub`
5. Развёртывает все сервисы
6. Применяет сетевые политики

### Ручная установка

Если ты хочешь понять, что происходит, выполни команды из `../danilpapa.md`.

## Структура файлов

```
k8s/
├── 01-redpanda.yaml          # Kafka broker (event streaming)
├── 02-redis.yaml             # In-memory cache и job store
├── 03-postgres.yaml          # Реляционная БД для job'ов
├── 04-minio.yaml             # Object storage (S3-compatible)
├── 05-clickhouse.yaml        # Analytics database
├── 06-jaeger.yaml            # Distributed tracing
├── 07-ai-service.yaml        # Python микросервис обработки мемов
├── 08-gateway.yaml           # Rust HTTP gateway
├── 09-network-policies.yaml  # Сетевые политики Cilium
├── setup-k3s-cilium.sh       # Автоматизированная установка
└── README.md                 # Этот файл
```

## Развёртывание

### Применить все манифесты по одному

```bash
kubectl apply -f ./01-redpanda.yaml
kubectl apply -f ./02-redis.yaml
kubectl apply -f ./03-postgres.yaml
kubectl apply -f ./04-minio.yaml
kubectl apply -f ./05-clickhouse.yaml
kubectl apply -f ./06-jaeger.yaml
kubectl apply -f ./07-ai-service.yaml
kubectl apply -f ./08-gateway.yaml
kubectl apply -f ./09-network-policies.yaml
```

### Или применить все сразу

```bash
kubectl apply -f ./
```

## Проверка статуса

```bash
# Смотрим все поды в namespace memehub
kubectl get pods -n memehub

# Смотрим подробный статус всех ресурсов
kubectl get all -n memehub

# Если что-то не работает, смотрим логи
kubectl logs -n memehub -l app=gateway -f

# Подробное описание пода
kubectl describe pod -n memehub <pod-name>
```

## Доступ к сервисам

### Port forwarding для локального доступа

```bash
# Gateway (основной API)
kubectl port-forward -n memehub svc/gateway 8080:8080 &

# Jaeger (трассировка)
kubectl port-forward -n memehub svc/jaeger 16686:16686 &

# MinIO (object storage console)
kubectl port-forward -n memehub svc/minio 9000:9000 9001:9001 &

# Redis
kubectl port-forward -n memehub svc/redis 6379:6379 &

# PostgreSQL
kubectl port-forward -n memehub svc/postgres 5432:5432 &
```

### URLs

- **Gateway API:** http://localhost:8080
- **Jaeger UI:** http://localhost:16686
- **MinIO Console:** http://localhost:9001 (admin/minioadmin)
- **Prometheus:** http://localhost:9090 (если развёрнут)

## Тестирование

### Проверить здоровье сервисов

```bash
# AI Service health
kubectl port-forward -n memehub svc/ai-service 8000:8000 &
curl http://localhost:8000/health

# Gateway metrics
curl http://localhost:8080/metrics

# Redpanda topics
kubectl exec -n memehub deployment/redpanda -- rpk topic list
```

### Отправить задачу на обработку

```bash
# С файлом
curl -X POST http://localhost:8080/ai/process \
  -F "file=@/path/to/meme.png"

# С URL
curl -X POST http://localhost:8080/ai/process \
  -H "Content-Type: application/json" \
  -d '{"image_url": "https://example.com/image.png"}'

# Проверить job статус (замени <job-id> на UUID из ответа выше)
curl http://localhost:8080/ai/jobs/<job-id>
```

## Сетевые политики (Cilium)

Файл `09-network-policies.yaml` содержит CiliumNetworkPolicy ресурсы, которые:

1. **Запрещают весь трафик по умолчанию** (deny-all)
2. **Разрешают конкретные потоки:**
   - External → Gateway (8080)
   - Gateway → AI Service (8000)
   - Gateway → Redis (6379)
   - Gateway & AI Service → Redpanda (9092)
   - Gateway & AI Service → PostgreSQL (5432)
   - Gateway & AI Service → MinIO (9000)
   - Gateway & AI Service → ClickHouse (8123)
   - Gateway → Jaeger (4317)
   - Все → DNS (53)

Это как firewall между микросервисами. Если сервис пытается обращаться куда-то, куда ему не разрешено, Cilium заблокирует запрос.

### Смотреть сетевые политики

```bash
kubectl get ciliumnetworkpolicies -n memehub
kubectl describe ciliumnetworkpolicies -n memehub allow-gateway-to-ai
```

### Hubble (Observability для Cilium)

```bash
# Включить Hubble UI (если не включён)
helm upgrade cilium cilium/cilium \
  --namespace kube-system \
  --reuse-values \
  --set hubble.relay.enabled=true \
  --set hubble.ui.enabled=true

# Смотреть Hubble UI
kubectl port-forward -n kube-system svc/hubble-ui 8081:80 &
```

Откроешь http://localhost:8081 и увидишь в реальном времени, какие поды общаются друг с другом!

## Мониторинг и Логирование

### Prometheus (если установлен отдельно)

```bash
kubectl port-forward -n memehub svc/prometheus 9090:9090 &
```

### Jaeger Traces

Gateway автоматически отправляет OpenTelemetry traces в Jaeger. Откроешь http://localhost:16686 и сможешь видеть полный путь запроса через сервисы.

## Обновление сервисов

### Обновить image deployment'а

```bash
# Если изменил код ai_service или gateway
docker compose build ai_service gateway

# Рестартить deployment (он подхватит новый image)
kubectl rollout restart deployment -n memehub ai-service
kubectl rollout restart deployment -n memehub gateway
```

### Масштабирование

```bash
# Запустить 3 копии gateway вместо 1
kubectl scale deployment -n memehub gateway --replicas=3

# Смотреть, что они работают
kubectl get pods -n memehub -l app=gateway
```

## Удаление

### Удалить всё

```bash
kubectl delete namespace memehub
```

Это удалит ВСЕ поды и ресурсы в namespace.

### Удалить k3s

```bash
sudo /usr/local/bin/k3s-uninstall.sh
```

## Проблемы и решения

### Все поды зависают в Pending

**Причина:** нет ресурсов на ноде

```bash
# Проверить ресурсы
kubectl describe nodes

# Уменьшить requests в манифесте
# например, изменить memory: "512Mi" на "256Mi"
```

### Pod в статусе CrashLoopBackOff

**Причина:** приложение падает при старте

```bash
# Смотреть логи
kubectl logs -n memehub <pod-name> -p

# Смотреть events
kubectl describe pod -n memehub <pod-name>
```

### Сервисы не видят друг друга

```bash
# Проверить DNS
kubectl exec -n memehub <pod-name> -- nslookup redis

# Проверить connectivity
kubectl exec -n memehub <pod-name> -- nc -zv redis 6379

# Проверить сетевые политики
kubectl get ciliumnetworkpolicies -n memehub
```

### Образ Docker не загружается

**Важно:** используется `imagePullPolicy: Never`, поэтому Docker images должны быть доступны локально.

```bash
# Собрать images
docker compose build ai_service gateway

# Проверить, что они есть
docker images | grep memehub
```

## Дополнительная информация

- Полный гайд для начинающих: `../danilpapa.md`
- Технология стека: `../rules/PROJECT_TECHNOLOGIES.md`
- Стратегия обновлений: `../rules/upgrade-strategy.md`

## Вопросы?

Смотри `../danilpapa.md` для подробного объяснения каждого компонента!
