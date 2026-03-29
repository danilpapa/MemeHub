# Как работать в проекте

### Перед запуском
запуск скрипта 
```bash
make onboarding
```
она установит: homebre, rustc, servicectl

### Добавление мироксервиса
сервисы добавлять в файл docker-compose.yml

### Сборка проекта
запуск команды 
```bash
make container
```
она: запустит servicectl (`https://github.com/danilpapa/servicectl`)
выбираем только те сервисы, исходный код которых изменялся -> она сама поднимет Docker 

### Альтернативный запуск
- Только приложение + Jaeger:
```bash
docker compose up --build -d
```
- Приложение + Jaeger + Prometheus + Grafana:
```bash
docker compose -f docker-compose.yml -f docker-compose.observability.yml up --build -d
```

Как проверить trace в UI:
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