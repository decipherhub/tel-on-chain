#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}Starting tel-indexer to fetch all blocks...${NC}"

# Run the indexer with the --fetch-all flag
RUST_BACKTRACE=1 cargo run --bin tel-indexer -- --fetch-all

EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}Successfully fetched all blocks.${NC}"
else
    echo -e "${RED}Failed to fetch blocks. Exit code: $EXIT_CODE${NC}"
fi

exit $EXIT_CODE
