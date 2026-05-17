# MemeHub Terraform Infrastructure

Terraform код для создания базовой инфраструктуры MemeHub в Kubernetes.

## Что создается

- ✅ **Namespaces** — изоляция ресурсов (memehub, ingress-nginx)
- ✅ **Service Accounts** — аккаунты для pods (gateway, ai-service, nginx-ingress)
- ✅ **Roles & RoleBindings** — RBAC политики доступа
- ✅ **Secrets** — зашифрованные пароли и ключи (postgres, minio, redis, aws-s3)

## Структура файлов

```
terraform/
├── main.tf              # Конфиг провайдера Kubernetes
├── namespaces.tf        # Создание namespaces
├── service_accounts.tf  # ServiceAccounts и RBAC
├── secrets.tf           # Secrets для приложений
├── variables.tf         # Переменные
├── outputs.tf           # Выходные значения
├── terraform.tfvars.example  # Пример переменных
├── .gitignore           # Игнорируемые файлы
└── README.md            # Этот файл
```

## Требования

- Terraform >= 1.0
- kubectl configured (файл ~/.kube/config)
- Kubernetes кластер должен быть запущен

## Как использовать

### 1. Инициализировать Terraform

```bash
cd terraform
terraform init
```

### 2. Посмотреть что будет создано

```bash
terraform plan
```

### 3. Применить конфиг

```bash
terraform apply
```

Когда попросит подтверждение - введи `yes`

### 4. Проверить что создалось

```bash
# Посмотреть namespaces
kubectl get namespaces

# Посмотреть Service Accounts
kubectl get serviceaccount -n memehub

# Посмотреть Secrets
kubectl get secrets -n memehub

# Посмотреть outputs от Terraform
terraform output
```

## Переменные

Скопируй `terraform.tfvars.example` в `terraform.tfvars` и отредактируй если нужно:

```bash
cp terraform.tfvars.example terraform.tfvars
# Отредактируй значения в terraform.tfvars
```

## Очистить все

Если нужно удалить все ресурсы:

```bash
terraform destroy
```

## Важно для production

- **Не коммитить** terraform.tfvars в git (он содержит пароли)
- Использовать более сильные пароли вместо значений по умолчанию
- Использовать remote state (S3, Terraform Cloud, и т.д.) вместо локального tfstate
- Добавить Terraform locking для team работы
