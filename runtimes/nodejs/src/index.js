/**
 * Node.js Runtime for Lambda Microservice
 *
 * This service executes JavaScript code submitted through the API.
 */

const express = require("express");
const bodyParser = require("body-parser");
const cors = require("cors");
const winston = require("winston");
const { VM } = require("vm2");
const { v4: uuidv4 } = require("uuid");

const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || "info",
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.json()
  ),
  transports: [new winston.transports.Console()],
});

const app = express();
const port = process.env.PORT || 8080;

app.use(bodyParser.json({ limit: "1mb" }));
app.use(cors());

app.get("/health", (req, res) => {
  res.status(200).json({ status: "ok" });
});

app.post("/execute", async (req, res) => {
  const startTime = Date.now();
  const { request_id, params, context, script_content } = req.body;

  if (!request_id) {
    return res.status(400).json({ error: "Missing request_id" });
  }

  logger.info(`Executing request ${request_id}`);

  try {
    const executionId = uuidv4();
    const memoryUsageStart = process.memoryUsage().heapUsed;

    let scriptToExecute;
    if (script_content) {
      scriptToExecute = script_content;
    } else {
      try {
        const languageTitle = context.language_title || "default";
        scriptToExecute = require(`./scripts/${languageTitle}.js`);
      } catch (err) {
        logger.error(
          `Failed to load script for language title ${
            context.language_title || "default"
          }: ${err.message}`
        );

        if (
          process.env.DB_LOGGING_ENABLED === "true" &&
          process.env.DB_CONNECTION_STRING
        ) {
          try {
            const { Pool } = require("pg");
            const pool = new Pool({
              connectionString: process.env.DB_CONNECTION_STRING,
            });

            await pool.query(
              `INSERT INTO public.error_logs 
              (request_log_id, error_code, error_message, stack_trace, context) 
              VALUES ($1, $2, $3, $4, $5)`,
              [
                request_id,
                "SCRIPT_NOT_FOUND",
                `Script not found for language title: ${
                  context.language_title || "default"
                }`,
                err.stack,
                JSON.stringify({ params, context }),
              ]
            );

            await pool.end();
          } catch (dbErr) {
            logger.error(`Failed to log error to database: ${dbErr.message}`);
          }
        }

        return res.status(404).json({
          error: `Script not found for language title: ${
            context.language_title || "default"
          }`,
        });
      }
    }

    const vm = new VM({
      timeout: parseInt(process.env.SCRIPT_TIMEOUT_MS || "5000"), // Configurable timeout
      sandbox: {
        console: {
          log: (...args) => logger.info(`[${request_id}] ${args.join(" ")}`),
          error: (...args) => logger.error(`[${request_id}] ${args.join(" ")}`),
          warn: (...args) => logger.warn(`[${request_id}] ${args.join(" ")}`),
        },
        process: {
          env: {
            NODE_ENV: process.env.NODE_ENV,
          },
        },
      },
    });

    let result;
    if (typeof scriptToExecute === "string") {
      const scriptFn = vm.run(`
        (async (event) => {
          ${scriptToExecute}
        })
      `);
      result = await scriptFn({ params, context });
    } else if (typeof scriptToExecute === "function") {
      result = await scriptToExecute({ params, context });
    } else {
      throw new Error("Invalid script format");
    }

    const executionTime = Date.now() - startTime;
    const memoryUsageEnd = process.memoryUsage().heapUsed;
    const memoryUsed = Math.max(0, memoryUsageEnd - memoryUsageStart);

    logger.info(
      `Request ${request_id} executed successfully in ${executionTime}ms`
    );

    if (
      process.env.DB_LOGGING_ENABLED === "true" &&
      process.env.DB_CONNECTION_STRING
    ) {
      try {
        const { Pool } = require("pg");
        const pool = new Pool({
          connectionString: process.env.DB_CONNECTION_STRING,
        });

        await pool.query(
          `INSERT INTO public.request_logs 
          (request_id, language_title, request_payload, response_payload, status_code, duration_ms, runtime_metrics) 
          VALUES ($1, $2, $3, $4, $5, $6, $7)`,
          [
            request_id,
            context.language_title || "default",
            JSON.stringify(params),
            JSON.stringify(result),
            200,
            executionTime,
            JSON.stringify({
              memory_usage_bytes: memoryUsed,
              memory_usage_mb:
                Math.round((memoryUsed / 1024 / 1024) * 100) / 100,
            }),
          ]
        );

        await pool.end();
      } catch (dbErr) {
        logger.error(`Failed to log execution to database: ${dbErr.message}`);
      }
    }

    res.status(200).json({
      result,
      execution_time_ms: executionTime,
      memory_usage_bytes: memoryUsed,
    });
  } catch (error) {
    logger.error(`Error executing request ${request_id}: ${error.message}`);
    logger.error(error.stack);

    if (
      process.env.DB_LOGGING_ENABLED === "true" &&
      process.env.DB_CONNECTION_STRING
    ) {
      try {
        const { Pool } = require("pg");
        const pool = new Pool({
          connectionString: process.env.DB_CONNECTION_STRING,
        });

        await pool.query(
          `INSERT INTO public.error_logs 
          (request_log_id, error_code, error_message, stack_trace, context) 
          VALUES ($1, $2, $3, $4, $5)`,
          [
            request_id,
            "EXECUTION_ERROR",
            error.message,
            error.stack,
            JSON.stringify({ params, context }),
          ]
        );

        await pool.end();
      } catch (dbErr) {
        logger.error(`Failed to log error to database: ${dbErr.message}`);
      }
    }

    res.status(500).json({
      error: error.message,
      execution_time_ms: Date.now() - startTime,
    });
  }
});

app.listen(port, () => {
  logger.info(`Node.js runtime listening on port ${port}`);
});

process.on("SIGTERM", () => {
  logger.info("SIGTERM received, shutting down");
  process.exit(0);
});

process.on("SIGINT", () => {
  logger.info("SIGINT received, shutting down");
  process.exit(0);
});
