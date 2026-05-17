# Упрощённый Terraform конфиг для MemeHub
# Требует: terraform init, terraform apply

terraform {
  required_version = ">= 1.0"
  required_providers {
    kubernetes = {
      source  = "hashicorp/kubernetes"
      version = "~> 2.23"
    }
  }
}

provider "kubernetes" {
  config_path = "~/.kube/config"
}

# Переменные
variable "memehub_namespace" {
  description = "Kubernetes namespace for MemeHub"
  default     = "memehub"
}

variable "postgres_password" {
  description = "PostgreSQL password"
  sensitive   = true
  default     = "memehub"
}

# 1. NAMESPACE
resource "kubernetes_namespace" "memehub" {
  metadata {
    name = var.memehub_namespace
    labels = {
      "app.kubernetes.io/name" = "memehub"
    }
  }
}

# 2. SERVICE ACCOUNTS
resource "kubernetes_service_account" "gateway" {
  metadata {
    name      = "gateway"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
}

resource "kubernetes_service_account" "ai_service" {
  metadata {
    name      = "ai-service"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
}

# 3. RBAC - Role
resource "kubernetes_role" "gateway" {
  metadata {
    name      = "gateway-role"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }

  rule {
    api_groups = [""]
    resources  = ["configmaps", "secrets"]
    verbs      = ["get", "list", "watch"]
  }
}

# 4. RBAC - RoleBinding
resource "kubernetes_role_binding" "gateway" {
  metadata {
    name      = "gateway-rolebinding"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }

  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "Role"
    name      = kubernetes_role.gateway.metadata[0].name
  }

  subject {
    kind      = "ServiceAccount"
    name      = kubernetes_service_account.gateway.metadata[0].name
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
}

# 5. SECRETS
resource "kubernetes_secret" "postgres" {
  metadata {
    name      = "postgres-credentials"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }

  type = "Opaque"
  data = {
    username = base64encode("memehub")
    password = base64encode(var.postgres_password)
    database = base64encode("memehub")
  }
}

resource "kubernetes_secret" "redis" {
  metadata {
    name      = "redis-credentials"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }

  type = "Opaque"
  data = {
    password = base64encode("redis")
  }
}

# OUTPUTS
output "namespace" {
  value = kubernetes_namespace.memehub.metadata[0].name
}

output "gateway_service_account" {
  value = kubernetes_service_account.gateway.metadata[0].name
}

output "postgres_secret" {
  value = kubernetes_secret.postgres.metadata[0].name
}
