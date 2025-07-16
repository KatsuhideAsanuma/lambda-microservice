/**
 * Node.js Runtime for Lambda Microservice
 * 
 * This service executes JavaScript code submitted through the API.
 */

const express = require('express');
const bodyParser = require('body-parser');
const cors = require('cors');
const winston = require('winston');
const { VM } = require('vm2');
const { v4: uuidv4 } = require('uuid');
const fs = require('fs');
const handler = require('./function/handler');

const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || 'info',
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.json()
  ),
  transports: [
    new winston.transports.Console()
  ]
});

const app = express();
const port = process.env.PORT || 8080;

app.use(bodyParser.json({ limit: '1mb' }));
app.use(cors());

app.get('/health', (req, res) => {
  res.status(200).json({ status: 'ok' });
});

app.post('/execute', async (req, res) => {
  const startTime = Date.now();
  const { request_id, params, context, script_content } = req.body;

  if (!request_id) {
    return res.status(400).json({ error: 'Missing request_id' });
  }

  logger.info(`Executing request ${request_id}`);

  try {
    const executionId = uuidv4();
    const memoryUsageStart = process.memoryUsage().heapUsed;
    
    let scriptToExecute;
    if (script_content) {
      scriptToExecute = script_content;
    } else {
      scriptToExecute = handler.toString();
    }

    const vm = new VM({
      timeout: 5000, // 5 seconds timeout
      sandbox: {
        console: {
          log: (...args) => logger.info(`[${request_id}] ${args.join(' ')}`),
          error: (...args) => logger.error(`[${request_id}] ${args.join(' ')}`),
          warn: (...args) => logger.warn(`[${request_id}] ${args.join(' ')}`)
        }
      }
    });

    let result;
    if (typeof scriptToExecute === 'string') {
      const scriptFn = vm.run(`
        (async (event) => {
          ${scriptToExecute}
        })
      `);
      result = await scriptFn({ params, context });
    } else if (typeof scriptToExecute === 'function') {
      result = await scriptToExecute({ params, context });
    } else {
      throw new Error('Invalid script format');
    }

    const executionTime = Date.now() - startTime;
    const memoryUsageEnd = process.memoryUsage().heapUsed;
    const memoryUsed = memoryUsageEnd - memoryUsageStart;

    logger.info(`Request ${request_id} executed successfully in ${executionTime}ms`);

    res.status(200).json({
      result,
      execution_time_ms: executionTime,
      memory_usage_bytes: memoryUsed
    });
  } catch (error) {
    logger.error(`Error executing request ${request_id}: ${error.message}`);
    logger.error(error.stack);

    res.status(500).json({
      error: error.message,
      execution_time_ms: Date.now() - startTime
    });
  }
});

process.env.handler = handler;

app.listen(port, () => {
  logger.info(`Node.js runtime listening on port ${port}`);
});

process.on('SIGTERM', () => {
  logger.info('SIGTERM received, shutting down');
  process.exit(0);
});

process.on('SIGINT', () => {
  logger.info('SIGINT received, shutting down');
  process.exit(0);
});
