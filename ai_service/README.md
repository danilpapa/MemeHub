# MemeHub AI Processing Service (local on macOS)

Минимальный сервис для OCR + тегов/эмоций.

## 1. Подготовка окружения

### Python
В macOS есть `python3` по пути `/usr/bin/python3`.

### Установка Tesseract (OCR)
```bash
brew install tesseract
```

## 2. Установка зависимостей
```bash
cd /Users/setuper/Desktop/UNI/3.2/arp/MemeHub/ai_service
python3 -m venv .venv
source .venv/bin/activate
pip install -r requirements.txt
```

## 3. Запуск
```bash
python3 main.py
```

Сервис поднимется на `http://localhost:8000`.

## 4. Проверка
```bash
curl http://localhost:8000/health
```

## 5. Пример запроса
### Вариант 1: загрузка файла
```bash
curl -X POST http://localhost:8000/process \
  -F "file=@/absolute/path/to/image.jpg"
```

### Вариант 2: URL картинки
```bash
curl -X POST http://localhost:8000/process \
  -H "Content-Type: application/json" \
  -d '{"image_url": "https://example.com/image.jpg"}'
```

## 6. Что дальше
- Подключить реальную LLM (Qwen 2.5) вместо заглушки `_simple_tags_emotion`.
- Добавить очередь (Kafka) и асинхронную обработку.
- Упаковать в Docker для деплоя.
