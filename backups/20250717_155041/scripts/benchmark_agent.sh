#!/bin/bash
set -e

CONTROLLER_URL=${CONTROLLER_URL:-"http://localhost:8080"}
DURATION=${DURATION:-"60"}  # Test duration in seconds
CONNECTIONS=${CONNECTIONS:-"100"}  # Number of connections
THREADS=${THREADS:-"8"}  # Number of threads

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Agent-Optimized Performance Benchmarking for Lambda Microservice${NC}"

if ! command -v wrk &> /dev/null
then
    echo -e "${YELLOW}Installing wrk...${NC}"
    sudo apt-get update
    sudo apt-get install -y build-essential libssl-dev git
    git clone https://github.com/wg/wrk.git
    cd wrk
    make
    sudo cp wrk /usr/local/bin
    cd ..
    rm -rf wrk
fi

cat > /tmp/benchmark_agent.lua << 'EOF'
-- Initialize the random number generator
math.randomseed(os.time())

-- Function to generate a random UUID
function generateUUID()
    local template = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'
    return string.gsub(template, '[xy]', function (c)
        local v = (c == 'x') and math.random(0, 0xf) or math.random(8, 0xb)
        return string.format('%x', v)
    end)
end

-- Initialize session with a simple JavaScript function
function init()
    local request_id = generateUUID()
    wrk.method = "POST"
    wrk.headers["Content-Type"] = "application/json"
    wrk.headers["Language-Title"] = "nodejs-calculator"
    
    -- Simple JavaScript calculator function
    local script = [[
    const operations = {
        add: (a, b) => a + b,
        subtract: (a, b) => a - b,
        multiply: (a, b) => a * b,
        divide: (a, b) => b !== 0 ? a / b : "Error: Division by zero"
    };
    
    const { operation, a, b } = event.params;
    
    if (!operations[operation]) {
        return { error: "Invalid operation" };
    }
    
    return { result: operations[operation](a, b) };
    ]]
    
    local body = '{"request_id":"' .. request_id .. '","context":{"env":"benchmark"},"script_content":"' .. script .. '"}'
    return wrk.format("POST", "/api/v1/initialize", wrk.headers, body)
end

-- Setup: Initialize sessions before benchmarking
function setup(thread)
    thread:set("initialized", false)
    thread:set("request_id", nil)
    thread:set("session_data", {})
    thread:set("latencies", {})
    thread:set("errors", 0)
    thread:set("requests", 0)
end

-- Request function: either initialize or execute
function request()
    if not wrk.thread:get("initialized") then
        -- Initialize first
        local response = init()
        wrk.thread:set("initialized", true)
        
        -- Extract request_id from response (simplified)
        wrk.thread:set("request_id", generateUUID())
        
        return response
    else
        -- Regular execution with the initialized session
        wrk.method = "POST"
        wrk.path = "/api/v1/execute"
        wrk.headers["Content-Type"] = "application/json"
        
        -- Random operation and numbers
        local operations = {"add", "subtract", "multiply", "divide"}
        local operation = operations[math.random(1, 4)]
        local a = math.random(1, 100)
        local b = math.random(1, 100)
        
        local body = '{"request_id":"' .. wrk.thread:get("request_id") .. '","params":{"operation":"' .. operation .. '","a":' .. a .. ',"b":' .. b .. '}}'
        
        return wrk.format("POST", nil, nil, body)
    end
end

-- Response handling
function response(status, headers, body, latency)
    local requests = wrk.thread:get("requests") + 1
    wrk.thread:set("requests", requests)
    
    -- Record latency
    local latencies = wrk.thread:get("latencies")
    table.insert(latencies, latency)
    wrk.thread:set("latencies", latencies)
    
    if status ~= 200 then
        local errors = wrk.thread:get("errors") + 1
        wrk.thread:set("errors", errors)
        print("Error: " .. status .. " - " .. body)
    end
end

-- Calculate percentiles
function percentile(sorted_array, p)
    if #sorted_array == 0 then
        return 0
    end
    local index = math.ceil(#sorted_array * p / 100)
    return sorted_array[math.min(index, #sorted_array)]
end

-- Done function to report statistics
function done(summary, latency, requests)
    local latencies = wrk.thread:get("latencies")
    local errors = wrk.thread:get("errors")
    local total_requests = wrk.thread:get("requests")
    
    -- Sort latencies for percentile calculation
    table.sort(latencies)
    
    -- Calculate percentiles
    local p50 = percentile(latencies, 50)
    local p90 = percentile(latencies, 90)
    local p95 = percentile(latencies, 95)
    local p99 = percentile(latencies, 99)
    
    -- Calculate error rate
    local error_rate = errors / total_requests * 100
    
    -- Print agent-specific metrics
    io.write("\n----- Agent-Optimized Metrics -----\n")
    io.write(string.format("Total Requests: %d\n", total_requests))
    io.write(string.format("Error Count: %d (%.2f%%)\n", errors, error_rate))
    io.write(string.format("Latency (ms):\n"))
    io.write(string.format("  p50: %.2f\n", p50))
    io.write(string.format("  p90: %.2f\n", p90))
    io.write(string.format("  p95: %.2f\n", p95))
    io.write(string.format("  p99: %.2f\n", p99))
    io.write("---------------------------------\n")
end
EOF

echo -e "${YELLOW}Starting agent-optimized benchmark...${NC}"
echo -e "URL: ${CONTROLLER_URL}"
echo -e "Duration: ${DURATION} seconds"
echo -e "Connections: ${CONNECTIONS}"
echo -e "Threads: ${THREADS}"
echo

wrk -t${THREADS} -c${CONNECTIONS} -d${DURATION}s -s /tmp/benchmark_agent.lua ${CONTROLLER_URL}

echo
echo -e "${GREEN}Benchmark completed!${NC}"

rm /tmp/benchmark_agent.lua
