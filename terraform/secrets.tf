# Secret для PostgreSQL
resource "kubernetes_secret" "postgres" {
  metadata {
    name      = "postgres-credentials"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }

  type = "Opaque"

  data = {
    username = base64encode("memehub")
    password = base64encode("memehub")
    database = base64encode("memehub")
  }
}

# Secret для MinIO
resource "kubernetes_secret" "minio" {
  metadata {
    name      = "minio-credentials"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }

  type = "Opaque"

  data = {
    access_key = base64encode("minioadmin")
    secret_key = base64encode("minioadmin")
  }
}

# Secret для Redis (если нужен пароль)
resource "kubernetes_secret" "redis" {
  metadata {
    name      = "redis-credentials"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }

  type = "Opaque"

  data = {
    password = base64encode("redis-password")
  }
}

# Secret для AWS S3 (если используется)
resource "kubernetes_secret" "aws_s3" {
  metadata {
    name      = "aws-s3-credentials"
    namespace = kubernetes_namespace.memehub.metadata[0].name
  }

  type = "Opaque"

  data = {
    access_key_id     = base64encode("AKIAIOSFODNN7EXAMPLE")
    secret_access_key = base64encode("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY")
  }
}
