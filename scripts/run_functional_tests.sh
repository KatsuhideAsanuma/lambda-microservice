#!/bin/bash
set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Running WebAssembly and gRPC Functional Tests${NC}"

chmod +x test_wasm_execution_extended.sh
chmod +x test_grpc_adapter.sh
chmod +x test_error_handling.sh

echo -e "\n${YELLOW}Running WebAssembly Execution and Redis Caching Test:${NC}"
./test_wasm_execution_extended.sh

echo -e "\n${YELLOW}Running gRPC Protocol Adapter Test:${NC}"
./test_grpc_adapter.sh

echo -e "\n${YELLOW}Running Error Handling and Retry Mechanism Test:${NC}"
./test_error_handling.sh

echo -e "\n${GREEN}All functional tests completed!${NC}"
