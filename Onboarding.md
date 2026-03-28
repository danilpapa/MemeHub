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