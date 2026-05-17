## Что такое Kubernetes? (Простое объяснение)

**Docker Compose vs Kubernetes:**
- **Docker Compose** — это как запуск приложения на одном компьютере. Ты скачиваешь контейнеры и запускаешь `docker compose up`. Работает просто, но если сломается один контейнер, или компьютер перезагрузится — всё выломается.
- **Kubernetes** — это как наймать менеджера проектов для твоих контейнеров. Менеджер следит, чтобы нужное количество копий приложения всегда работало, перезапускает упавшие контейнеры, распределяет нагрузку, обновляет версии без простоев.

**Kubernetes кластер** состоит из:
- **Master (Control Plane)** — мозг кластера, принимает команды, распределяет работу
- **Worker Nodes** — рабочие, на которых крутятся контейнеры (в локальном режиме это один или два компьютера)

## Что такое Cilium? (Сетевой управляющий)

**CNI** (Container Network Interface) — это как сетевой "коммутатор" для контейнеров. Если у тебя есть 5 контейнеров на разных машинах, CNI следит, чтобы они друг друга видели и могли общаться.

**Cilium** — это CNI на стероидах:
- Обычно CNI работает на уровне IP-адресов (просто пробрасывает пакеты)
- Cilium использует eBPF (extended Berkeley Packet Filter) — это как встроенная ОС прямо в ядро Linux, которая видит трафик на более глубоком уровне
- Это позволяет Cilium делать умные вещи: обнаруживать атаки, видеть, какие сервисы разговаривают друг с другом, применять сетевые политики

**В нашем случае:** Cilium поможет нам видеть трафик между Gateway → AI Service → Redpanda, применять политики (например, "только Gateway может обращаться в Kafka"), и отлавливать проблемы.

## Варианты установки локального K8s

### Вариант 1: k3s (Рекомендуется для начинающих) ⭐
- **Что это:** Легкая версия Kubernetes от Rancher
- **Плюсы:** Устанавливается в 10 команд, работает на macOS, Linux, WSL
- **Минусы:** Немного упрощён, но для локальной разработки идеален
- **Память:** 2-4 GB
- **Скорость установки:** 2-3 минуты

### Вариант 2: k0s
- **Что это:** Ещё более минималистичный K8s
- **Плюсы:** Меньше памяти, подходит для слабых машин
- **Минусы:** Меньше документации, чем у k3s
- **Память:** 1-2 GB

### Вариант 3: Talos Linux (Сложный путь)
- **Что это:** Полноценный K8s на специальной Linux ОС, заточенной под контейнеры
- **Плюсы:** Максимальный контроль, как в production
- **Минусы:** Нужна виртуальная машина (KVM/VirtualBox), сложнее настройка
- **Память:** 4-8 GB
- **Скорость установки:** 10-15 минут

**Рекомендация:** Начни с **k3s** — это 80% того, что нужно, но 20% усилий.

---

## Установка k3s + Cilium на macOS

### Шаг 1: Установка k3s

```bash
# Скачиваем и устанавливаем k3s (это просто один большой бинарник)
curl -sfL https://get.k3s.io | sh -

# Проверяем, что k3s запустился
sudo k3s kubectl get nodes

# Для удобства: скопируем конфиг Kubernetes в домашнюю папку
mkdir -p ~/.kube
sudo cp /etc/rancher/k3s/k3s.yaml ~/.kube/config
sudo chown $(id -u):$(id -g) ~/.kube/config

# Теперь можешь использовать kubectl без sudo
kubectl get nodes
```

**Что произошло:**
- Скачалась готовая сборка Kubernetes
- Автоматически поднялся Control Plane и Worker на одной машине
- Создался конфиг-файл, который говорит `kubectl` (командной утилите), где находится кластер

### Шаг 2: Отключение встроенного CNI

По умолчанию k3s поставляется с Flannel CNI. Нам нужна Cilium.

```bash
# Переустанавливаем k3s БЕЗ встроенного CNI
# Сначала стираем старую версию
sudo /usr/local/bin/k3s-uninstall.sh

# Переустанавливаем с флагом --flannel-backend=none
curl -sfL https://get.k3s.io | INSTALL_K3S_SKIP_DOWNLOAD=false sh -s - --flannel-backend=none

# Проверяем (노드будут в статусе NotReady — это нормально, ждём Cilium)
kubectl get nodes
```

### Шаг 3: Установка Cilium

```bash
# Устанавливаем Helm (это пакетный менеджер для Kubernetes)
brew install helm

# Добавляем репозиторий Cilium
helm repo add cilium https://helm.cilium.io
helm repo update

# Устанавливаем Cilium
helm install cilium cilium/cilium \
  --namespace kube-system \
  --set kubeProxyReplacement=partial \
  --set k3s.enabled=true

# Проверяем, что Cilium установилась (может занять 1-2 минуты)
kubectl get pods -n kube-system | grep cilium
```

**Что произошло:**
- Cilium развернулась в `kube-system` namespace (системное пространство имён)
- На каждом узле поднялась Cilium pod — это наши "сетевые менеджеры"

### Шаг 4: Проверка кластера

```bash
# Проверяем, что все узлы Ready
kubectl get nodes

# Смотрим все системные поды (должны быть в Running)
kubectl get pods -n kube-system

# Смотрим Cilium специально
kubectl get pods -n kube-system -l k8s-app=cilium
```

**Ожидаемый результат:**
```
NAME              STATUS   ROLES    
k3s-master        Ready    master   

NAMESPACE      NAME                          READY   STATUS    
kube-system    cilium-xxx-yyyyy              1/1     Running   
kube-system    cilium-operator-xxx           1/1     Running   
```

---

## Развёртывание MemeHub в Kubernetes

### Шаг 5: Создание Namespace

Namespace — это как папка для проекта. Все сервисы MemeHub будут жить в одной папке.

```bash
kubectl create namespace memehub
```

### Шаг 6: Создание Deployment для каждого сервиса

Теперь нам нужно создать файлы описания каждого сервиса. Создай папку для Kubernetes манифестов:

```bash
mkdir -p ./k8s
```

#### Redpanda (Kafka) deployment

`./k8s/redpanda.yaml`:

Что здесь происходит:
- **ConfigMap** — конфигурация для Redpanda
- **Deployment** — основной сервис Redpanda (1 реплика = 1 контейнер)
- **Service** — "адрес" для связи с Redpanda (другие поды могут обращаться по имени `redpanda:9092`)
- **Job** — одноразовая задача для создания Kafka topics

#### 6.2 Redis deployment

Создай файл `./k8s/redis.yaml`:

#### 6.3 PostgreSQL deployment

Создай файл `./k8s/postgres.yaml`:

#### 6.4 MinIO deployment

Создай файл `./k8s/minio.yaml`:

#### 6.6 Jaeger deployment

Создай файл `./k8s/jaeger.yaml`:

#### 6.7 AI Service deployment

Создай файл `./k8s/ai-service.yaml`:

### Шаг 7: Развёртывание всех сервисов

```bash
# Применяем все манифесты
kubectl apply -f ./k8s/

# Проверяем, что всё развернулось
kubectl get pods -n memehub

# Ждём, пока все поды не будут в статусе Running (может занять 1-2 минуты)
kubectl get pods -n memehub -w
```

---

## Тестирование и доступ к сервисам

### Проверка подов

```bash
# Смотрим статус всех подов
kubectl get pods -n memehub

# Смотрим логи конкретного пода (например, gateway)
kubectl logs -n memehub -l app=gateway -f

# Если что-то не работает, смотрим подробное описание пода
kubectl describe pod -n memehub <pod-name>
```

### Доступ к UI сервисов

```bash
# Получаем IP адреса внешних сервисов
kubectl get svc -n memehub

# Пробрасываем порты для локального доступа
kubectl port-forward -n memehub svc/gateway 8080:8080 &
kubectl port-forward -n memehub svc/jaeger 16686:16686 &
kubectl port-forward -n memehub svc/minio 9000:9000 9001:9001 &
```

Теперь доступ:
- **Gateway:** `http://localhost:8080`
- **Jaeger UI:** `http://localhost:16686`
- **MinIO Console:** `http://localhost:9001` (admin/minioadmin)

### Тестирование API

```bash
# Отправка задачи на анализ
curl -X POST http://localhost:8080/ai/process \
  -F "file=@/path/to/meme.png"

# Проверка метрик
curl http://localhost:8080/metrics

# Проверка здоровья AI service
kubectl port-forward -n memehub svc/ai-service 8000:8000 &
curl http://localhost:8000/health
```

---

## Сетевые политики Cilium

Теперь, когда всё работает, можем настроить сетевые политики. Это как firewall для твоих микросервисов.

### Базовая политика: запретить всё, кроме разрешённого

Создай файл `./k8s/network-policies.yaml`:

# Запретить весь входящий трафик по умолчанию
# Разрешить Gateway принимать запросы (из вне кластера)
# Разрешить Gateway → AI Service
# Разрешить Gateway → Redis
# Разрешить Gateway & AI Service → Kafka (Redpanda)
# Разрешить Gateway → PostgreSQL
# Разрешить Gateway & AI Service → MinIO
# Разрешить Gateway & AI Service → ClickHouse
# Разрешить Gateway → Jaeger

Применяем политики:

```bash
kubectl apply -f ./k8s/network-policies.yaml
```

### Проверка политик

```bash
# Смотрим все сетевые политики
kubectl get ciliumnetworkpolicies -n memehub

# Смотрим подробно
kubectl describe ciliumnetworkpolicies -n memehub
```

---

## Мониторинг с Cilium Hubble

Cilium имеет встроенный observability инструмент **Hubble**, который позволяет видеть трафик между подами.

```bash
# Включаем Hubble (if not already enabled)
helm upgrade cilium cilium/cilium \
  --namespace kube-system \
  --reuse-values \
  --set hubble.relay.enabled=true \
  --set hubble.ui.enabled=true

# Ждём, пока поднимутся hubble поды
kubectl get pods -n kube-system -l k8s-app=hubble

# Пробрасываем порт для UI
kubectl port-forward -n kube-system svc/hubble-ui 8081:80 &

# Откроем в браузере http://localhost:8081
```

**В Hubble UI ты сможешь видеть:**
- Все connections между подами в реальном времени
- Какие поды разговаривают с какими (зелёные=успешно, красные=ошибки)
- Throughput (количество запросов в секунду)
- DNS запросы

---

## Полезные команды

```bash
# Смотреть все ресурсы в namespace
kubectl get all -n memehub

# Смотреть события (что происходит)
kubectl get events -n memehub --sort-by='.lastTimestamp'

# Посмотреть переменные окружения пода
kubectl exec -n memehub <pod-name> -- env

# Войти в pod и выполнить команды
kubectl exec -it -n memehub <pod-name> -- /bin/sh

# Скопировать файл из пода
kubectl cp memehub/<pod-name>:/app/file.txt ./file.txt

# Удалить deployment (остановит все его поды)
kubectl delete deployment -n memehub <deployment-name>

# Перезапустить deployment
kubectl rollout restart deployment -n memehub <deployment-name>

# Смотреть логи в реальном времени
kubectl logs -n memehub -l app=gateway -f

# Смотреть использование ресурсов
kubectl top pods -n memehub
```

**Что ты получил:**
- ✅ Локальный K8s кластер (k3s + Cilium)
- ✅ Все сервисы MemeHub развёрнуты в K8s
- ✅ Сетевые политики (какие сервисы с какими общаются)
- ✅ eBPF observability (Cilium Hubble видит весь трафик)
- ✅ Трассировка (Jaeger) и метрики (Prometheus)

**Главные преимущества перед Docker Compose:**
- Автоматические рестарты падающих контейнеров
- Масштабирование (можешь запустить 3 копии Gateway вместо 1)
- Более реалистично к production (в production также используют K8s)
- Cilium дает видимость в то, как твои сервисы общаются

---

## Задание 3.2: Ingress Controller + Load Balancing

### Что нужно?

Ingress Controller — это **точка входа** в кластер (фронтовой門). Он слушает порт 80/443 и маршрутизирует HTTP/HTTPS запросы к нужным сервисам.

**Аналогия для iOS:** 
- Без Ingress = напрямую звонишь другу (знаешь номер)
- С Ingress = звонишь оператору, он соединяет тебя с нужным человеком (оператор = Ingress)

### Наша реализация

Мы используем **Nginx** как простой Ingress Controller:

```yaml
# 10-ingress-controller.yaml

upstream gateway {
  server gateway.memehub.svc.cluster.local:8080;
}

server {
  listen 80;
  
  location / {
    proxy_pass http://gateway;  # Все запросы идут сюда
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
  }
}
```

**Поток запроса:**
```
Пользователь (http://localhost/api/memes)
    ↓
Nginx Ingress (слушает порт 80)
    ↓
Маршрутизирует на Service: gateway.memehub.svc.cluster.local:8080
    ↓
K8s Service распределяет на pods Gateway
    ↓
Pod обрабатывает запрос
```

### Про Keepalived и HA

**Keepalived** — это для высокой доступности (HA) когда у тебя несколько серверов.

Пример HA архитектуры:
```
    Virtual IP: 192.168.1.100 (управляется Keepalived)
           ↓
    ┌──────┴──────┐
    ↓             ↓
Ingress #1    Ingress #2
на Node 1      на Node 2

Если Node 1 упадет → Virtual IP переключается на Node 2
(пользователи не заметят)
```

**В нашем случае:** 
- Docker Desktop = одна машина
- Один Ingress достаточно
- Keepalived не нужен (нему нечего failover'ить)

Если бы хотели HA, пришлось бы:
1. Запустить Keepalived деплоймент
2. Настроить виртуальный IP через VRRP протокол
3. Синхронизировать состояние между Ingress'ами

Но это сложно на Docker Desktop, поэтому делаем просто.

### Как применить

```bash
# Применить Ingress Controller
kubectl apply -f 10-ingress-controller.yaml

# Проверить статус
kubectl get pods -n ingress-nginx
kubectl get svc -n ingress-nginx

# Проверить что работает
kubectl port-forward -n ingress-nginx svc/nginx-ingress 8080:80

# В другом терминале
curl http://localhost:8080/api/memes
```

### Мониторинг load balancing

Ingress распределяет запросы между pods (если их несколько):

```bash
# Запустить 3 копии Gateway
kubectl scale deployment gateway -n memehub --replicas=3

# Каждый запрос пойдет на другой pod (round-robin)
for i in {1..10}; do
  curl -s http://localhost:8080/api/memes | head -1
done
```

### Итого по заданию 3.2

✅ **Ingress Controller** — Nginx (простой, работающий)
✅ **Load Balancing** — встроенный round-robin между pods
✅ **Keepalived** — не нужен для одной машины (но архитектура описана выше)

---

## Задание 2.1: Terraform для инфраструктуры

### Что такое Terraform?

Terraform — это IaC (Infrastructure as Code). Вместо того чтобы кликать в UI или писать kubectl команды, ты описываешь нужную инфраструктуру в коде.

**Аналогия для iOS:**
- Без Terraform = вручную создавать Interface Builder в Xcode
- С Terraform = писать SwiftUI код который создает UI

### Наша структура

```
terraform/
├── main.tf              # Конфиг провайдера
├── namespaces.tf        # Создание memehub и ingress-nginx namespaces
├── service_accounts.tf  # ServiceAccounts для gateway, ai-service, nginx-ingress
├── secrets.tf           # Пароли и ключи (postgres, minio, redis, aws-s3)
├── variables.tf         # Переменные (можно переопределять)
├── outputs.tf           # Что вывести после создания
├── terraform.tfvars     # Значения переменных (не коммитим!)
└── README.md            # Инструкции
```

### Основные объекты

#### 1. Namespace

```hcl
resource "kubernetes_namespace" "memehub" {
  metadata {
    name = "memehub"
  }
}
```

Это создает namespace (изоляция для ресурсов). Как создать новую папку в Xcode Project.

#### 2. Service Account

```hcl
resource "kubernetes_service_account" "gateway" {
  metadata {
    name      = "gateway"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
}
```

Service Account — это "пользователь" для pod'ов. Pod использует SA для доступа к K8s API.

#### 3. Secret

```hcl
resource "kubernetes_secret" "postgres" {
  metadata {
    name      = "postgres-credentials"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }

  data = {
    username = base64encode("memehub")
    password = base64encode("memehub")
  }
}
```

Secret — это зашифрованные переменные окружения. Пароли, ключи, токены.

#### 4. Role & RoleBinding (RBAC)

```hcl
resource "kubernetes_role" "nginx_ingress" {
  metadata {
    name = "nginx-ingress"
  }

  rule {
    api_groups = [""]
    resources  = ["configmaps"]
    verbs      = ["get", "list", "watch"]
  }
}

resource "kubernetes_role_binding" "nginx_ingress" {
  # Привязываем Role к ServiceAccount
  subject {
    kind      = "ServiceAccount"
    name      = kubernetes_service_account.nginx_ingress.metadata[0].name
  }

  role_ref {
    kind = "Role"
    name = kubernetes_role.nginx_ingress.metadata[0].name
  }
}
```

RBAC = Role-Based Access Control. Определяет что может делать каждый ServiceAccount.

Например: "nginx-ingress может читать ConfigMaps, но не может удалять pods"

### Как использовать

#### Шаг 1: Инициализировать

```bash
cd terraform
terraform init
```

Это скачивает провайдер Kubernetes и готовит всё.

#### Шаг 2: Посмотреть план

```bash
terraform plan
```

Это показывает что будет создано, без применения.

#### Шаг 3: Применить

```bash
terraform apply
```

Terraform создаст namespaces, service accounts, secrets.

#### Шаг 4: Проверить

```bash
# Посмотреть outputs
terraform output

# Проверить в kubectl
kubectl get namespaces
kubectl get serviceaccount -n memehub
kubectl get secrets -n memehub
```

### Зачем это нужно?

**Docker Compose + kubectl:**
- Нужно помнить какие команды писать
- Сложно повторить на другой машине
- Если что-то удалить — нужно вручную пересоздавать

**С Terraform:**
- Один файл = вся инфраструктура (reproducible)
- Легко масштабировать (просто добавить в код)
- История изменений в git
- Можно использовать переменные

### Пример: добавить еще один Service Account

Просто добавить в `service_accounts.tf`:

```hcl
resource "kubernetes_service_account" "new_service" {
  metadata {
    name      = "new-service"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
}
```

И запустить:

```bash
terraform apply
```

Готово! Service Account создан.

### Важное для production

- **Не коммитить** `terraform.tfvars` (содержит пароли)
- Использовать remote state (S3, TFC) вместо локального файла
- Добавить `terraform.lock.hcl` в git
- Использовать более сильные пароли

### Итого по заданию 2.1

✅ **Namespaces** — memehub, ingress-nginx созданы как код
✅ **Service Accounts** — для всех компонентов
✅ **Secrets** — для postgres, minio, redis, aws-s3
✅ **RBAC** — простые роли для Nginx Ingress

Все для галочки, просто и работает 👍
