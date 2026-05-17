output "memehub_namespace" {
  description = "MemeHub namespace name"
  value       = kubernetes_namespace.memehub.metadata[0].name
}

output "ingress_nginx_namespace" {
  description = "Ingress Nginx namespace name"
  value       = kubernetes_namespace.ingress_nginx.metadata[0].name
}

output "gateway_service_account" {
  description = "Gateway service account name"
  value       = kubernetes_service_account.gateway.metadata[0].name
}

output "ai_service_account" {
  description = "AI Service account name"
  value       = kubernetes_service_account.ai_service.metadata[0].name
}

output "postgres_secret_name" {
  description = "PostgreSQL secret name"
  value       = kubernetes_secret.postgres.metadata[0].name
}

output "minio_secret_name" {
  description = "MinIO secret name"
  value       = kubernetes_secret.minio.metadata[0].name
}

output "redis_secret_name" {
  description = "Redis secret name"
  value       = kubernetes_secret.redis.metadata[0].name
}

output "aws_s3_secret_name" {
  description = "AWS S3 secret name"
  value       = kubernetes_secret.aws_s3.metadata[0].name
}
