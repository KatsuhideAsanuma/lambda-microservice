#!/bin/bash
set -e

DB_HOST=${DB_HOST:-"localhost"}
DB_PORT=${DB_PORT:-"5432"}
DB_NAME=${DB_NAME:-"lambda_microservice"}
DB_USER=${DB_USER:-"postgres"}
DB_PASSWORD=${DB_PASSWORD:-"postgres"}
MIGRATIONS_DIR=${MIGRATIONS_DIR:-"$(dirname $0)/../database/migrations"}

function show_help {
    echo "Database Migration Utility"
    echo "Usage: $0 [options]"
    echo "Options:"
    echo "  -h, --host        Database host (default: localhost)"
    echo "  -p, --port        Database port (default: 5432)"
    echo "  -d, --database    Database name (default: lambda_microservice)"
    echo "  -u, --user        Database user (default: postgres)"
    echo "  -w, --password    Database password (default: postgres)"
    echo "  -m, --migrations  Migrations directory (default: ../database/migrations)"
    echo "  --help            Show this help message"
}

while [[ $# -gt 0 ]]; do
    key="$1"
    case $key in
        -h|--host)
            DB_HOST="$2"
            shift
            shift
            ;;
        -p|--port)
            DB_PORT="$2"
            shift
            shift
            ;;
        -d|--database)
            DB_NAME="$2"
            shift
            shift
            ;;
        -u|--user)
            DB_USER="$2"
            shift
            shift
            ;;
        -w|--password)
            DB_PASSWORD="$2"
            shift
            shift
            ;;
        -m|--migrations)
            MIGRATIONS_DIR="$2"
            shift
            shift
            ;;
        --help)
            show_help
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

export PGPASSWORD="$DB_PASSWORD"

function execute_sql_file {
    local file=$1
    echo "Executing migration: $file"
    psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -f "$file"
}

function get_version_from_filename {
    local filename=$(basename "$1")
    echo "$filename" | grep -oP '^V\K[0-9]+\.[0-9]+\.[0-9]+'
}

function get_description_from_filename {
    local filename=$(basename "$1")
    echo "$filename" | sed -E 's/^V[0-9]+\.[0-9]+\.[0-9]+__(.*)\.sql$/\1/' | tr '_' ' '
}

echo "Starting database migration..."

psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d postgres -tc "SELECT 1 FROM pg_database WHERE datname = '$DB_NAME'" | grep -q 1 || psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d postgres -c "CREATE DATABASE $DB_NAME"

psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
    CREATE SCHEMA IF NOT EXISTS meta;
    CREATE TABLE IF NOT EXISTS meta.schema_version (
        id SERIAL PRIMARY KEY,
        version VARCHAR(32) NOT NULL,
        description TEXT NOT NULL,
        applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
        applied_by VARCHAR(128) NOT NULL
    );
"

APPLIED_VERSIONS=$(psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -t -c "SELECT version FROM meta.schema_version ORDER BY id;")

for file in $(find "$MIGRATIONS_DIR" -name "V*.sql" | sort); do
    VERSION=$(get_version_from_filename "$file")
    DESCRIPTION=$(get_description_from_filename "$file")
    
    if echo "$APPLIED_VERSIONS" | grep -q "$VERSION"; then
        echo "Version $VERSION already applied, skipping $file"
        continue
    fi
    
    execute_sql_file "$file"
    
    psql -h "$DB_HOST" -p "$DB_PORT" -U "$DB_USER" -d "$DB_NAME" -c "
        INSERT INTO meta.schema_version (version, description, applied_by) 
        VALUES ('$VERSION', '$DESCRIPTION', 'migration_script');
    "
    
    echo "Successfully applied migration $VERSION: $DESCRIPTION"
done

echo "Database migration completed successfully."
