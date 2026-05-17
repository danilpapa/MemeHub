variable "memehub_namespace" {
  description = "Kubernetes namespace for MemeHub"
  type        = string
  default     = "memehub"
}

variable "postgres_username" {
  description = "PostgreSQL username"
  type        = string
  default     = "memehub"
  sensitive   = true
}

variable "postgres_password" {
  description = "PostgreSQL password"
  type        = string
  default     = "memehub"
  sensitive   = true
}

variable "minio_access_key" {
  description = "MinIO access key"
  type        = string
  default     = "minioadmin"
  sensitive   = true
}

variable "minio_secret_key" {
  description = "MinIO secret key"
  type        = string
  default     = "minioadmin"
  sensitive   = true
}
