# Service Account для Gateway
resource "kubernetes_service_account" "gateway" {
  metadata {
    name      = "gateway"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
}

# Service Account для AI Service
resource "kubernetes_service_account" "ai_service" {
  metadata {
    name      = "ai-service"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }
}

# Service Account для Nginx Ingress Controller
resource "kubernetes_service_account" "nginx_ingress" {
  metadata {
    name      = "nginx-ingress"
    namespace = kubernetes_namespace.ingress_nginx.metadata[0].name
  }
}

# Role для Nginx Ingress (может читать ConfigMaps)
resource "kubernetes_role" "nginx_ingress" {
  metadata {
    name      = "nginx-ingress"
    namespace = kubernetes_namespace.ingress_nginx.metadata[0].name
  }

  rule {
    api_groups = [""]
    resources  = ["configmaps"]
    verbs      = ["get", "list", "watch"]
  }
}

# RoleBinding для Nginx Ingress
resource "kubernetes_role_binding" "nginx_ingress" {
  metadata {
    name      = "nginx-ingress"
    namespace = kubernetes_namespace.ingress_nginx.metadata[0].name
  }

  role_ref {
    api_group = "rbac.authorization.k8s.io"
    kind      = "Role"
    name      = kubernetes_role.nginx_ingress.metadata[0].name
  }

  subject {
    kind      = "ServiceAccount"
    name      = kubernetes_service_account.nginx_ingress.metadata[0].name
    namespace = kubernetes_namespace.ingress_nginx.metadata[0].name
  }
}
