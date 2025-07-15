const http = require('http');
const port = process.env.PORT || 8081;
const responseDelayMs = parseInt(process.env.RESPONSE_DELAY_MS || '50');
const errorRate = parseFloat(process.env.ERROR_RATE || '0.1');

const server = http.createServer((req, res) => {
  if (req.method === 'POST' && req.url === '/execute') {
    let body = '';
    req.on('data', chunk => {
      body += chunk.toString();
    });
    
    req.on('end', () => {
      setTimeout(() => {
        if (Math.random() < errorRate) {
          res.writeHead(500, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'Simulated runtime error' }));
          return;
        }
        
        try {
          const requestData = JSON.parse(body);
          
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({
            result: { 
              output: `Executed ${requestData.request_id} with params ${JSON.stringify(requestData.params)}`,
              language: 'nodejs'
            },
            execution_time_ms: responseDelayMs,
            memory_usage_bytes: 1024 * 1024
          }));
        } catch (err) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'Invalid request data' }));
        }
      }, responseDelayMs);
    });
  } else if (req.method === 'GET' && req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'ok', runtime: 'nodejs' }));
  } else {
    res.writeHead(404, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ error: 'Not found' }));
  }
});

server.listen(port, () => {
  console.log(`Mock NodeJS runtime server running on port ${port}`);
});
