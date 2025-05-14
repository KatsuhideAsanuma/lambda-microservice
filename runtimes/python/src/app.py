"""
Python Runtime for Lambda Microservice

This service executes Python code submitted through the API.
"""

import os
import time
import uuid
import json
import traceback
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

@app.get("/api/v1/health")
async def health():
    return {
        "status": "ok",
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    }

@app.post("/execute")
async def execute(request: ExecuteRequest):
    start_time = time.time()
    logger.info(f"Executing request {request.request_id}")

    try:
        execution_id = str(uuid.uuid4())
        
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
            exec(script_to_execute, globals_dict)
            
            if "handle" in globals_dict and callable(globals_dict["handle"]):
                result = globals_dict["handle"](request.params)
            else:
                result = globals_dict.get("result")

        stdout_output = stdout_buffer.getvalue()
        stderr_output = stderr_buffer.getvalue()
        
        if stdout_output:
            logger.info(f"[{request.request_id}] STDOUT: {stdout_output}")
        if stderr_output:
            logger.warning(f"[{request.request_id}] STDERR: {stderr_output}")

        execution_time = int((time.time() - start_time) * 1000)  # Convert to ms
        
        logger.info(f"Request {request.request_id} executed successfully in {execution_time}ms")

        return ExecuteResponse(
            result=result,
            execution_time_ms=execution_time,
            memory_usage_bytes=None
        )
    
    except Exception as e:
        logger.error(f"Error executing request {request.request_id}: {str(e)}")
        logger.error(traceback.format_exc())
        
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
