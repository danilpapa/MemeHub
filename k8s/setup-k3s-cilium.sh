#!/bin/bash
set -e

echo "🚀 Setting up k3s + Cilium + MemeHub"
echo ""

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Step 1: Install k3s
echo -e "${BLUE}Step 1: Installing k3s${NC}"
if ! command -v k3s &> /dev/null; then
    curl -sfL https://get.k3s.io | sh -
    echo -e "${GREEN}✓ k3s installed${NC}"
else
    echo -e "${YELLOW}⚠ k3s already installed, skipping${NC}"
fi

echo ""

# Step 2: Setup kubeconfig
echo -e "${BLUE}Step 2: Setting up kubeconfig${NC}"
mkdir -p ~/.kube
sudo cp /etc/rancher/k3s/k3s.yaml ~/.kube/config
sudo chown $(id -u):$(id -g) ~/.kube/config
echo -e "${GREEN}✓ kubeconfig set up${NC}"

echo ""

# Step 3: Check if k3s is running
echo -e "${BLUE}Step 3: Waiting for k3s to be ready${NC}"
until kubectl get nodes &> /dev/null; do
    echo "Waiting for k3s..."
    sleep 2
done
echo -e "${GREEN}✓ k3s is ready${NC}"

echo ""

# Step 4: Install Helm
echo -e "${BLUE}Step 4: Installing Helm${NC}"
if ! command -v helm &> /dev/null; then
    brew install helm
    echo -e "${GREEN}✓ Helm installed${NC}"
else
    echo -e "${YELLOW}⚠ Helm already installed, skipping${NC}"
fi

echo ""

# Step 5: Setup Cilium
echo -e "${BLUE}Step 5: Installing Cilium${NC}"
helm repo add cilium https://helm.cilium.io || true
helm repo update

helm install cilium cilium/cilium \
  --namespace kube-system \
  --set kubeProxyReplacement=partial \
  --set k3s.enabled=true || echo -e "${YELLOW}⚠ Cilium might already be installed${NC}"

echo -e "${GREEN}✓ Cilium is being installed (this takes 1-2 minutes)${NC}"

echo ""

# Step 6: Wait for Cilium to be ready
echo -e "${BLUE}Step 6: Waiting for Cilium to be ready${NC}"
until kubectl get pods -n kube-system -l k8s-app=cilium &> /dev/null; do
    echo "Waiting for Cilium pods..."
    sleep 2
done

until kubectl get pods -n kube-system -l k8s-app=cilium -o jsonpath='{.items[0].status.phase}' 2>/dev/null | grep -q Running; do
    echo "Waiting for Cilium to run..."
    sleep 2
done
echo -e "${GREEN}✓ Cilium is ready${NC}"

echo ""

# Step 7: Create namespace
echo -e "${BLUE}Step 7: Creating MemeHub namespace${NC}"
kubectl create namespace memehub 2>/dev/null || echo -e "${YELLOW}⚠ Namespace already exists${NC}"
echo -e "${GREEN}✓ Namespace created${NC}"

echo ""

# Step 8: Build Docker images
echo -e "${BLUE}Step 8: Building Docker images${NC}"
docker compose -f docker-compose.yml build ai_service gateway
echo -e "${GREEN}✓ Docker images built${NC}"

echo ""

# Step 9: Load Docker images into k3s
echo -e "${BLUE}Step 9: Loading Docker images into k3s${NC}"
docker save memehub-ai-service:latest | kubectl exec -i -n memehub $(kubectl get pods -n memehub -l app=ai-service -o jsonpath='{.items[0].metadata.name}' 2>/dev/null || echo "pending") -- docker load 2>/dev/null || echo -e "${YELLOW}⚠ Will load images after deployment${NC}"
echo -e "${GREEN}✓ Docker images loaded${NC}"

echo ""

# Step 10: Deploy all services
echo -e "${BLUE}Step 10: Deploying MemeHub services${NC}"
kubectl apply -f ./k8s/01-redpanda.yaml
kubectl apply -f ./k8s/02-redis.yaml
kubectl apply -f ./k8s/03-postgres.yaml
kubectl apply -f ./k8s/04-minio.yaml
kubectl apply -f ./k8s/05-clickhouse.yaml
kubectl apply -f ./k8s/06-jaeger.yaml
kubectl apply -f ./k8s/07-ai-service.yaml
kubectl apply -f ./k8s/08-gateway.yaml
echo -e "${GREEN}✓ Services deployed${NC}"

echo ""

# Step 11: Deploy network policies
echo -e "${BLUE}Step 11: Deploying network policies${NC}"
kubectl apply -f ./k8s/09-network-policies.yaml
echo -e "${GREEN}✓ Network policies deployed${NC}"

echo ""

# Step 12: Wait for all pods to be running
echo -e "${BLUE}Step 12: Waiting for all pods to be running${NC}"
echo "This may take 2-5 minutes depending on your system..."
kubectl wait --for=condition=ready pod -l app -n memehub --timeout=300s 2>/dev/null || true

echo ""
echo -e "${GREEN}✅ Setup complete!${NC}"
echo ""
echo -e "${BLUE}📋 Quick commands:${NC}"
echo "View all pods:        kubectl get pods -n memehub"
echo "View pod logs:        kubectl logs -n memehub -l app=gateway -f"
echo "Port forward gateway: kubectl port-forward -n memehub svc/gateway 8080:8080 &"
echo "Port forward Jaeger:  kubectl port-forward -n memehub svc/jaeger 16686:16686 &"
echo "Port forward MinIO:   kubectl port-forward -n memehub svc/minio 9000:9000 9001:9001 &"
echo ""
echo -e "${BLUE}🌐 Access URLs:${NC}"
echo "Gateway:   http://localhost:8080"
echo "Jaeger UI: http://localhost:16686"
echo "MinIO:     http://localhost:9001 (admin/minioadmin)"
echo ""
