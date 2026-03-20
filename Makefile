run:
	@chmod +x scripts/open_docker.sh && ./scripts/open_docker.sh
	@docker compose up --build

prev:
	@chmod +x scripts/open_docker.sh && ./scripts/open_docker.sh
	@docker compose up
	