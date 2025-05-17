FROM node:18-alpine

WORKDIR /app

# Install PostgreSQL client for database logging
RUN apk add --no-cache postgresql-client

# Install dependencies
COPY package.json package-lock.json ./
RUN npm ci --production

# Copy application code
COPY . .

# Expose port
EXPOSE 8080

# Set environment variables
ENV NODE_ENV=production

# Run the application
CMD ["node", "src/index.js"]
