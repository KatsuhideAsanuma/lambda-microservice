"""
Python Runtime for Lambda Microservice

This service executes Python code submitted through the API.
"""

import os
import time
import uuid
import json
import traceback
import psutil
from typing import Dict, Any, Optional
from contextlib import redirect_stdout, redirect_stderr
import io
import sys

from fastapi import FastAPI, HTTPException, Request
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel
from loguru import logger

logger.remove()
logger.add(sys.stderr, level=os.environ.get("LOG_LEVEL", "INFO"))

app = FastAPI(title="Python Runtime for Lambda Microservice")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

class ExecuteRequest(BaseModel):
    request_id: str
    params: Dict[str, Any]
    context: Dict[str, Any]
    script_content: Optional[str] = None

class ExecuteResponse(BaseModel):
    result: Any
    execution_time_ms: int
    memory_usage_bytes: Optional[int] = None

@app.get("/health")
async def health():
    return {"status": "ok"}

@app.post("/execute")
async def execute(request: ExecuteRequest):
    start_time = time.time()
    logger.info(f"Executing request {request.request_id}")

    try:
        execution_id = str(uuid.uuid4())
        
        # Get memory usage before execution
        process = psutil.Process(os.getpid())
        memory_before = process.memory_info().rss
        
        script_to_execute = None
        if request.script_content:
            script_to_execute = request.script_content
        else:
            language_title = request.context.get("language_title", "default")
            try:
                script_path = f"./scripts/{language_title}.py"
                with open(script_path, "r") as f:
                    script_to_execute = f.read()
            except FileNotFoundError:
                if os.environ.get("DB_LOGGING_ENABLED") == "true":
                    try:
                        import psycopg2
                        import json
                        conn = psycopg2.connect(os.environ.get("DB_CONNECTION_STRING"))
                        cur = conn.cursor()
                        
                        cur.execute(
                            """INSERT INTO public.error_logs 
                            (request_log_id, error_code, error_message, context) 
                            VALUES (%s, %s, %s, %s)""",
                            (
                                request.request_id,
                                "SCRIPT_NOT_FOUND",
                                f"Script not found for language title: {language_title}",
                                json.dumps({"params": request.params, "context": request.context})
                            )
                        )
                        
                        conn.commit()
                        cur.close()
                        conn.close()
                    except Exception as db_err:
                        logger.error(f"Failed to log error to database: {str(db_err)}")
                
                raise HTTPException(
                    status_code=404,
                    detail=f"Script not found for language title: {language_title}"
                )

        globals_dict = {
            "params": request.params,
            "context": request.context,
            "logger": logger,
            "request_id": request.request_id,
        }

        stdout_buffer = io.StringIO()
        stderr_buffer = io.StringIO()

        result = None
        with redirect_stdout(stdout_buffer), redirect_stderr(stderr_buffer):
            try:
                exec(script_to_execute, globals_dict)
                
                if "handle" in globals_dict and callable(globals_dict["handle"]):
                    result = globals_dict["handle"](request.params)
                else:
                    result = globals_dict.get("result")
            except Exception as exec_error:
                logger.error(f"Script execution error: {str(exec_error)}")
                logger.error(traceback.format_exc())
                
                if os.environ.get("DB_LOGGING_ENABLED") == "true":
                    try:
                        import psycopg2
                        import json
                        conn = psycopg2.connect(os.environ.get("DB_CONNECTION_STRING"))
                        cur = conn.cursor()
                        
                        cur.execute(
                            """INSERT INTO public.error_logs 
                            (request_log_id, error_code, error_message, stack_trace, context) 
                            VALUES (%s, %s, %s, %s, %s)""",
                            (
                                request.request_id,
                                "SCRIPT_EXECUTION_ERROR",
                                str(exec_error),
                                traceback.format_exc(),
                                json.dumps({"params": request.params, "context": request.context})
                            )
                        )
                        
                        conn.commit()
                        cur.close()
                        conn.close()
                    except Exception as db_err:
                        logger.error(f"Failed to log error to database: {str(db_err)}")
                
                raise exec_error

        stdout_output = stdout_buffer.getvalue()
        stderr_output = stderr_buffer.getvalue()
        
        if stdout_output:
            logger.info(f"[{request.request_id}] STDOUT: {stdout_output}")
        if stderr_output:
            logger.warning(f"[{request.request_id}] STDERR: {stderr_output}")

        execution_time = int((time.time() - start_time) * 1000)  # Convert to ms
        
        # Get memory usage after execution
        memory_after = process.memory_info().rss
        memory_used = max(0, memory_after - memory_before)
        
        if os.environ.get("DB_LOGGING_ENABLED") == "true":
            try:
                import psycopg2
                import json
                conn = psycopg2.connect(os.environ.get("DB_CONNECTION_STRING"))
                cur = conn.cursor()
                
                cur.execute(
                    """INSERT INTO public.request_logs 
                    (request_id, language_title, request_payload, response_payload, status_code, duration_ms) 
                    VALUES (%s, %s, %s, %s, %s, %s)""",
                    (
                        request.request_id,
                        request.context.get("language_title", "default"),
                        json.dumps(request.params),
                        json.dumps(result),
                        200,
                        execution_time
                    )
                )
                
                conn.commit()
                cur.close()
                conn.close()
            except Exception as db_err:
                logger.error(f"Failed to log execution to database: {str(db_err)}")
        
        logger.info(f"Request {request.request_id} executed successfully in {execution_time}ms")

        return ExecuteResponse(
            result=result,
            execution_time_ms=execution_time,
            memory_usage_bytes=memory_used
        )
    
    except Exception as e:
        logger.error(f"Error executing request {request.request_id}: {str(e)}")
        logger.error(traceback.format_exc())
        
        if os.environ.get("DB_LOGGING_ENABLED") == "true":
            try:
                import psycopg2
                import json
                conn = psycopg2.connect(os.environ.get("DB_CONNECTION_STRING"))
                cur = conn.cursor()
                
                cur.execute(
                    """INSERT INTO public.error_logs 
                    (request_log_id, error_code, error_message, stack_trace, context) 
                    VALUES (%s, %s, %s, %s, %s)""",
                    (
                        request.request_id,
                        "REQUEST_EXECUTION_ERROR",
                        str(e),
                        traceback.format_exc(),
                        json.dumps({"params": request.params, "context": request.context})
                    )
                )
                
                conn.commit()
                cur.close()
                conn.close()
            except Exception as db_err:
                logger.error(f"Failed to log error to database: {str(db_err)}")
        
        raise HTTPException(
            status_code=500,
            detail={
                "error": str(e),
                "execution_time_ms": int((time.time() - start_time) * 1000)
            }
        )

if __name__ == "__main__":
    import uvicorn
    port = int(os.environ.get("PORT", 8080))
    uvicorn.run("app:app", host="0.0.0.0", port=port, reload=False)
