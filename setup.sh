#!/usr/bin/env bash
# CRUST one-liner setup script
# Usage: bash setup.sh
set -e

GREEN='\033[0;32m'
BOLD='\033[1m'
RESET='\033[0m'

echo -e "${BOLD}🦀 CRUST Setup${RESET}\n"

# Check required tools
for cmd in docker cargo git; do
  if ! command -v "$cmd" &>/dev/null; then
    echo "❌ '$cmd' not found. Install it first."
    case $cmd in
      docker) echo "   https://docs.docker.com/get-docker/" ;;
      cargo)  echo "   https://rustup.rs" ;;
    esac
    exit 1
  fi
done

echo "✅ All prerequisites found"

# If not already inside the repo, clone it
if [ ! -f "Cargo.toml" ]; then
  echo -e "\n${GREEN}▶ Cloning repository...${RESET}"
  git clone https://github.com/bhaumiksoni/crust.git
  cd crust
fi

# Start the server (PostgreSQL + crust-server)
echo -e "\n${GREEN}▶ Starting server (Docker)...${RESET}"
docker compose up -d --build

# Wait for the server to be ready
echo -n "   Waiting for server"
for i in {1..30}; do
  if curl -sf http://localhost:8080/health &>/dev/null; then
    echo " ready!"
    break
  fi
  echo -n "."
  sleep 1
done

# Install the CLI
echo -e "\n${GREEN}▶ Installing crust CLI...${RESET}"
cargo install --path crust-cli --force

echo -e "\n${BOLD}✅ Done!${RESET}"
echo ""
echo "  Server : http://localhost:8080"
echo "  CLI    : $(which crust 2>/dev/null || echo 'crust') — run \`crust --version\` to verify"
echo ""
echo -e "${BOLD}Next steps:${RESET}"
echo "  1. crust login http://localhost:8080"
echo "  2. crust init my-project && cd my-project"
echo "  3. crust commit -m 'first commit'"
echo "  4. crust push"
echo ""
