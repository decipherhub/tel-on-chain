#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}Starting all TEL services...${NC}"

# Function to cleanup background processes
cleanup() {
    echo -e "\n${YELLOW}Shutting down all services...${NC}"
    jobs -p | xargs -r kill
    exit 0
}

# Set up trap to cleanup on exit
trap cleanup SIGINT SIGTERM

# Start tel-api
echo -e "${BLUE}Starting tel-api...${NC}"
cargo run --bin tel-api &
API_PID=$!

# Start tel-indexer
echo -e "${BLUE}Starting tel-indexer...${NC}"
cargo run --bin tel-indexer &
INDEXER_PID=$!

# Start tel-ui (Rust binary)
echo -e "${BLUE}Starting tel-ui...${NC}"
cargo run --bin tel-ui &
UI_PID=$!

# Start frontend (Next.js)
echo -e "${BLUE}Starting frontend (Next.js)...${NC}"
cd tel-ui-web && npm run dev &
FRONTEND_PID=$!
cd ..

echo -e "${GREEN}All services started!${NC}"
echo -e "${YELLOW}Services running:${NC}"
echo -e "  - tel-api (PID: $API_PID)"
echo -e "  - tel-indexer (PID: $INDEXER_PID)"
echo -e "  - tel-ui (PID: $UI_PID)"
echo -e "  - frontend (PID: $FRONTEND_PID)"
echo -e "${YELLOW}Press Ctrl+C to stop all services${NC}"

# Wait for all background processes
wait