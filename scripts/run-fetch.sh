#!/bin/bash

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if "full" argument is passed
if [ "$1" = "full" ]; then
    echo -e "${BLUE}Starting tel-indexer to fetch all pools...${NC}"
    cargo run --bin tel-indexer -- --fetch-all
    FETCH_TYPE="all pools"
else
    echo -e "${BLUE}Starting tel-indexer to fetch light mode pools...${NC}"
    echo -e "${BLUE}(Use './run-fetch.sh full' to fetch all pools)${NC}"
    cargo run --bin tel-indexer -- --fetch-light
    FETCH_TYPE="light mode pools"
fi

EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}Successfully fetched ${FETCH_TYPE}.${NC}"
else
    echo -e "${RED}Failed to fetch ${FETCH_TYPE}. Exit code: $EXIT_CODE${NC}"
fi

exit $EXIT_CODE
