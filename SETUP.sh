#!/bin/bash

set -e

PROJECT_ROOT="/Users/danilzabinskij/Desktop/uni/arp/MemeHub/MemeHub"
cd "$PROJECT_ROOT"

echo "================================"
echo "🚀 MemeHub Setup"
echo "================================"

# Step 1: Start Docker services
echo ""
echo "📦 Step 1: Запускаем Redis, Kafka, Jaeger..."
docker-compose up -d
sleep 5

echo "✅ Docker сервисы запущены"
docker-compose ps

# Step 2: Check services are healthy
echo ""
echo "🏥 Step 2: Проверяем что сервисы здоровы..."
echo "⏳ Ждём 10 секунд для инициализации Kafka..."
sleep 10

# Check Redis
if docker exec memehub-redis-1 redis-cli ping | grep -q PONG; then
    echo "✅ Redis OK"
else
    echo "❌ Redis НЕ отвечает"
fi

# Check Kafka
if docker exec memehub-kafka-1 kafka-broker-api-versions --bootstrap-server localhost:9092 &>/dev/null; then
    echo "✅ Kafka OK"
else
    echo "⚠️  Kafka инициализируется, попробуем через 5 секунд..."
    sleep 5
fi

echo ""
echo "================================"
echo "🎯 Готово к запуску Gateway!"
echo "================================"
echo ""
echo "Теперь в НОВОМ терминале окне выполни:"
echo ""
echo "  cd $PROJECT_ROOT/gateway"
echo "  cargo run"
echo ""
echo "Gateway будет доступен на: http://localhost:8080"
echo ""
echo "Проверить здоровье:"
echo "  curl http://localhost:8080/health"
echo ""
echo "Остановить всё:"
echo "  docker-compose down"
