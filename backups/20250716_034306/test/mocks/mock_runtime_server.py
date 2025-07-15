import http.server
import json
import os
import time
import random
from urllib.parse import urlparse

PORT = int(os.environ.get('PORT', 8082))
RESPONSE_DELAY_MS = int(os.environ.get('RESPONSE_DELAY_MS', 50))
ERROR_RATE = float(os.environ.get('ERROR_RATE', 0.1))

class MockRuntimeHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == '/health':
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'status': 'ok', 'runtime': 'python'}).encode())
        else:
            self.send_response(404)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'error': 'Not found'}).encode())
    
    def do_POST(self):
        if self.path == '/execute':
            content_length = int(self.headers['Content-Length'])
            post_data = self.rfile.read(content_length)
            
            time.sleep(RESPONSE_DELAY_MS / 1000.0)
            
            if random.random() < ERROR_RATE:
                self.send_response(500)
                self.send_header('Content-type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({'error': 'Simulated runtime error'}).encode())
                return
            
            try:
                request_data = json.loads(post_data.decode('utf-8'))
                
                self.send_response(200)
                self.send_header('Content-type', 'application/json')
                self.end_headers()
                
                response = {
                    'result': {
                        'output': f"Executed {request_data['request_id']} with params {json.dumps(request_data['params'])}",
                        'language': 'python'
                    },
                    'execution_time_ms': RESPONSE_DELAY_MS,
                    'memory_usage_bytes': 1024 * 1024
                }
                
                self.wfile.write(json.dumps(response).encode())
            except Exception as e:
                self.send_response(400)
                self.send_header('Content-type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({'error': f'Invalid request data: {str(e)}'}).encode())
        else:
            self.send_response(404)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({'error': 'Not found'}).encode())

if __name__ == '__main__':
    server = http.server.HTTPServer(('', PORT), MockRuntimeHandler)
    print(f'Mock Python runtime server running on port {PORT}')
    server.serve_forever()
