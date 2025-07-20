#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to show usage
show_usage() {
    echo -e "${BLUE}Usage: $0 [OPTIONS]${NC}"
    echo "Options:"
    echo "  test              Enable test mode (indexer will use test pools)"
    echo "  --port PORT       Specify API server port (default: 8081)"
    echo "  -p PORT           Specify API server port (default: 8081)"
    echo "  --help, -h        Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0                    # Start with default port 8081"
    echo "  $0 --port 3000       # Start API on port 3000"
    echo "  $0 test --port 3000   # Test mode on port 3000"
}

echo -e "${GREEN}Starting TEL API and Indexer services...${NC}"

# Function to cleanup background processes
cleanup() {
    echo -e "\n${YELLOW}Shutting down API and Indexer services...${NC}"
    jobs -p | xargs -r kill
    exit 0
}

# Set up trap to cleanup on exit
trap cleanup SIGINT SIGTERM

# Parse arguments
TEST_MODE=0
API_PORT=""

while [[ $# -gt 0 ]]; do
    case $1 in
        test)
            TEST_MODE=1
            echo -e "${YELLOW}Running in TEST MODE (indexer will use test pools)!${NC}"
            shift
            ;;
        --port|-p)
            if [[ -n "$2" && "$2" =~ ^[0-9]+$ ]]; then
                API_PORT="$2"
                echo -e "${YELLOW}API will run on port $API_PORT${NC}"
                shift 2
            else
                echo -e "${RED}Error: --port requires a valid port number${NC}"
                show_usage
                exit 1
            fi
            ;;
        --help|-h)
            show_usage
            exit 0
            ;;
        *)
            echo -e "${RED}Error: Unknown option $1${NC}"
            show_usage
            exit 1
            ;;
    esac
done

# Set API port environment variable if specified
if [[ -n "$API_PORT" ]]; then
    export TEL_API_PORT="$API_PORT"
fi

# Start tel-api
echo -e "${BLUE}Starting tel-api...${NC}"
cargo run --bin tel-api &
API_PID=$!

# Start tel-indexer
echo -e "${BLUE}Starting tel-indexer...${NC}"
if [[ $TEST_MODE -eq 1 ]]; then
    cargo run --bin tel-indexer -- --test-mode &
else
    cargo run --bin tel-indexer &
fi
INDEXER_PID=$!

echo -e "${GREEN}API and Indexer services started!${NC}"
echo -e "${YELLOW}Services running:${NC}"
if [[ -n "$API_PORT" ]]; then
    echo -e "  - tel-api (PID: $API_PID) on port $API_PORT"
else
    echo -e "  - tel-api (PID: $API_PID) on default port 8081"
fi
echo -e "  - tel-indexer (PID: $INDEXER_PID)"
echo -e "${YELLOW}Press Ctrl+C to stop services${NC}"

# Wait for all background processes
wait
