onboarding: 
	@chmod +x ./scripts/setup.sh && ./scripts/setup.sh

update:
	@cargo install --git https://github.com/danilpapa/servicectl --force

container:
	@servicectl

scratch:
	@chmod +x scripts/open_docker.sh && ./scripts/open_docker.sh
	@docker compose up --build

check_cilium:
	@kubectl get pods -n kube-system | grep ciliuma

pods: 
	@kubectl get pods -n memehub -w