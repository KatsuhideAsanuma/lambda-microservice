FROM node:18-alpine

WORKDIR /app

# Install dependencies
COPY package.json ./
RUN npm install --production

# Copy application code
COPY . .

# Expose port
EXPOSE 8080

# Set environment variables
ENV NODE_ENV=production

# Run the application
CMD ["node", "src/index.js"]
