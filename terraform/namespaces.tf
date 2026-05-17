# Namespace для MemeHub
resource "kubernetes_namespace" "memehub" {
  metadata {
    name = "memehub"
    labels = {
      "app.kubernetes.io/name" = "memehub"
    }
  }
}

# Namespace для Ingress Controller
resource "kubernetes_namespace" "ingress_nginx" {
  metadata {
    name = "ingress-nginx"
    labels = {
      "app.kubernetes.io/name" = "ingress-nginx"
    }
  }
}
