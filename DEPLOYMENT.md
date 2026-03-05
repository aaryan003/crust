# CRUST Deployment Guide

**Last Updated**: 2026-03-05  
**Version**: 1.0.0

---

## Overview

CRUST is deployed as a containerized application using Docker and Docker Compose. This guide covers:
- Setting up the deployment environment
- Running CRUST locally with Docker Compose
- Deploying to production
- Monitoring and troubleshooting

---

## Prerequisites

- **Docker** 20.10+ ([install](https://docs.docker.com/get-docker/))
- **Docker Compose** 2.0+ ([install](https://docs.docker.com/compose/install/))
- **Git** 2.30+ (for cloning the repo)
- **Make** (optional, for convenience)

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU Cores | 2 | 4+ |
| RAM | 2GB | 4GB+ |
| Disk Space | 5GB | 20GB+ |
| OS | Linux, macOS, Windows (WSL2) | Linux |

---

## Quick Start (Development)

### 1. Clone the repository

```bash
git clone https://github.com/crust/crust.git
cd crust
```

### 2. Create environment file

```bash
cp .env.example .env
```

Edit `.env` and set custom values (or use defaults for local testing):

```dotenv
# Database credentials
POSTGRES_USER=crust_user
POSTGRES_PASSWORD=crust_password
POSTGRES_DB=crust

# JWT secret (generate with: openssl rand -base64 32)
JWT_SECRET=your-super-secret-jwt-key-here-minimum-64-characters-long

# Server configuration
PORT=8080
LOG_LEVEL=info

# Repository storage
REPO_BASE_PATH=/data/repos

# JWT token expiry
JWT_EXPIRY_SECONDS=86400

# Registration settings
ALLOW_REGISTRATION=true
```

### 3. Start the services

```bash
docker-compose up
```

This will:
- Build the Docker image for `crust-server`
- Start PostgreSQL 16 (port 5432)
- Run migrations automatically
- Start the API server (port 8080)

### 4. Verify the deployment

```bash
# Check health
curl http://localhost:8080/health

# Expected response:
# {"status":"ok","service":"crust-server","database":{"connected":true}}
```

### 5. Stop the services

```bash
docker-compose down
```

To also remove volumes (cleanup):

```bash
docker-compose down -v
```

---

## Production Deployment

### 1. Generate secure JWT secret

```bash
# Generate 64-character random string
openssl rand -base64 64

# Or use openssl to generate 32 bytes -> 44 chars base64
JWT_SECRET=$(openssl rand -base64 32)
echo "JWT_SECRET=$JWT_SECRET" >> .env.production
```

### 2. Create production environment file

```bash
cp .env.example .env.production
```

Update `.env.production` with production values:

```dotenv
# Use strong database credentials
POSTGRES_USER=crust_prod_user
POSTGRES_PASSWORD=$(openssl rand -base64 32)
POSTGRES_DB=crust_prod

# Secure JWT secret
JWT_SECRET=$(openssl rand -base64 64)

# Security settings
ALLOW_REGISTRATION=false  # Enable user registration via admin only

# Logging
LOG_LEVEL=warn  # Reduce log verbosity in production

# Performance
JWT_EXPIRY_SECONDS=604800  # 7 days
```

### 3. Deploy with docker-compose

```bash
# Use production environment file
docker-compose --env-file .env.production up -d
```

Or deploy to a cloud provider (see "Cloud Deployment" section below).

### 4. Verify production deployment

```bash
# Check container status
docker-compose ps

# View logs
docker-compose logs -f app

# Health check
curl https://your-domain.com/health
```

---

## Database Management

### Run migrations manually

Migrations run automatically on startup, but you can run them manually:

```bash
# Inside the running container
docker-compose exec app sqlx migrate run --database-url $DATABASE_URL
```

Or directly with PostgreSQL:

```bash
# From host machine
PGPASSWORD=crust_password psql -h localhost -U crust_user -d crust -f crust-server/migrations/001_initial_schema.sql
```

### Backup the database

```bash
# Dump database
docker-compose exec db pg_dump -U crust_user crust > backup.sql

# Restore from backup
docker-compose exec db psql -U crust_user crust < backup.sql
```

### Access PostgreSQL shell

```bash
docker-compose exec db psql -U crust_user -d crust
```

---

## Monitoring & Logs

### View logs

```bash
# All services
docker-compose logs

# Only app
docker-compose logs app

# Only database
docker-compose logs db

# Follow logs
docker-compose logs -f app
```

### Monitor container resources

```bash
# Real-time stats
docker stats

# Or use individual commands
docker stats crust-server crust-postgres
```

### Health checks

The deployment includes automatic health checks:

```bash
# Manual health check
curl http://localhost:8080/health

# Response example:
{
  "status": "ok",
  "service": "crust-server",
  "version": "0.1.0",
  "timestamp": "2026-03-05T10:30:45.123456Z",
  "database": {
    "connected": true,
    "response_time_ms": 2,
    "pool_size": 10
  }
}
```

---

## Troubleshooting

### Issue: "Connection refused"

```bash
# Check if containers are running
docker-compose ps

# Check logs
docker-compose logs app
docker-compose logs db

# Ensure database is healthy
docker-compose logs db | grep "ready to accept"
```

**Solution**: Wait for database startup (30+ seconds) before connecting.

### Issue: "Database connection failed"

```bash
# Verify DATABASE_URL is correct
docker-compose exec app env | grep DATABASE_URL

# Test connection manually
docker-compose exec db psql -U crust_user -d crust -c "SELECT 1"
```

**Solution**: Check .env file has correct credentials and database name.

### Issue: "Port already in use"

```bash
# Find process using port 8080
lsof -i :8080

# Or change port in .env
PORT=8081 docker-compose up
```

**Solution**: Change port in .env or stop conflicting service.

### Issue: "Out of disk space"

```bash
# Clean Docker resources
docker system prune -a

# Or just remove unused volumes
docker volume prune
```

**Solution**: Backup and clean PostgreSQL volumes if needed.

### Issue: "Migrations failed"

```bash
# Check migration files
ls -la crust-server/migrations/

# View database state
docker-compose exec db psql -U crust_user -d crust -c "\dt"
```

**Solution**: Ensure migration files exist and database user has permission.

---

## Docker Compose Reference

### Build from scratch

```bash
docker-compose build --no-cache
```

### Pull latest base images

```bash
docker-compose pull
```

### Scale services (not recommended for this setup)

```bash
# Scale app (don't do this without load balancer)
docker-compose up --scale app=3
```

### Remove everything

```bash
docker-compose down -v --remove-orphans
```

---

## Cloud Deployment

### AWS ECS

```bash
# Push image to ECR
aws ecr get-login-password --region us-east-1 | \
  docker login --username AWS --password-stdin 123456789.dkr.ecr.us-east-1.amazonaws.com

docker tag crust-server:latest 123456789.dkr.ecr.us-east-1.amazonaws.com/crust-server:latest
docker push 123456789.dkr.ecr.us-east-1.amazonaws.com/crust-server:latest

# Deploy with ECS CLI (or Terraform)
```

### Heroku

```bash
# Create app
heroku create crust-app

# Set environment variables
heroku config:set JWT_SECRET=$(openssl rand -base64 64) -a crust-app

# Deploy (requires Heroku PostgreSQL add-on)
git push heroku main
```

### Kubernetes

```yaml
# deployment.yaml example
apiVersion: apps/v1
kind: Deployment
metadata:
  name: crust-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: crust
  template:
    metadata:
      labels:
        app: crust
    spec:
      containers:
      - name: crust-server
        image: crust-server:latest
        ports:
        - containerPort: 8080
        env:
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: crust-secrets
              key: database_url
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
```

Deploy with `kubectl apply -f deployment.yaml`.

---

## Environment Variables Reference

| Variable | Default | Required | Description |
|----------|---------|----------|-------------|
| `DATABASE_URL` | postgres://crust_user:crust_password@db:5432/crust | Yes | PostgreSQL connection string |
| `JWT_SECRET` | (none) | Yes | Secret for signing JWTs (min 64 chars) |
| `PORT` | 8080 | No | Server port |
| `LOG_LEVEL` | info | No | Logging level (debug, info, warn, error) |
| `REPO_BASE_PATH` | /data/repos | No | Path for object storage |
| `JWT_EXPIRY_SECONDS` | 86400 | No | JWT token expiry (seconds) |
| `ALLOW_REGISTRATION` | true | No | Allow new user registration |
| `RUST_LOG` | crust_server=info | No | Rust log filter (see `tracing-subscriber`) |

---

## Performance Tuning

### PostgreSQL Connection Pool

Adjust in `crust-server/src/database.rs`:

```rust
sqlx::postgres::PgPoolOptions::new()
    .max_connections(20)  // Increase for high throughput
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(5))
    .connect(&database_url)
    .await?
```

### Axum Server Concurrency

Set in environment:

```bash
# Tokio thread count
TOKIO_WORKER_THREADS=8
```

### PostgreSQL Tuning

Add to `docker-compose.yml` postgres service:

```yaml
command:
  - "postgres"
  - "-c"
  - "max_connections=200"
  - "-c"
  - "shared_buffers=256MB"
  - "-c"
  - "effective_cache_size=1GB"
```

---

## SSL/TLS Setup

### With Let's Encrypt (nginx-proxy)

```yaml
# docker-compose.yml with nginx-proxy
services:
  proxy:
    image: nginxproxy/nginx-proxy:latest
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - /var/run/docker.sock:/tmp/docker.sock:ro
      - certs:/etc/nginx/certs

  app:
    # ... existing config
    environment:
      VIRTUAL_HOST: crust.example.com
      LETSENCRYPT_HOST: crust.example.com
      LETSENCRYPT_EMAIL: admin@example.com
```

### Manual SSL certificate

```bash
# Generate self-signed cert (testing only)
openssl req -x509 -newkey rsa:4096 -keyout key.pem -out cert.pem -days 365 -nodes

# Use in nginx or reverse proxy
```

---

## Backup & Recovery

### Automated backups

```bash
#!/bin/bash
# backup.sh - daily database backup

BACKUP_DIR="/backups/crust"
DATE=$(date +%Y%m%d_%H%M%S)

mkdir -p "$BACKUP_DIR"

docker-compose exec -T db pg_dump \
  -U crust_user \
  crust | gzip > "$BACKUP_DIR/crust_$DATE.sql.gz"

# Keep only last 30 days
find "$BACKUP_DIR" -name "*.sql.gz" -mtime +30 -delete
```

Schedule with cron:

```bash
0 2 * * * /path/to/backup.sh
```

### Full recovery procedure

```bash
# 1. Stop services
docker-compose down

# 2. Remove database volume (dangerous!)
docker volume rm crust_postgres_data

# 3. Start database again
docker-compose up -d db

# 4. Wait for health check
sleep 30

# 5. Restore backup
gunzip < crust_backup.sql.gz | docker-compose exec -T db psql -U crust_user crust

# 6. Start app
docker-compose up -d app
```

---

## Maintenance

### Regular checks

```bash
# Weekly: Check logs for errors
docker-compose logs --since 7d app | grep -i error

# Monthly: Prune Docker resources
docker system prune -a

# Quarterly: Update base images
docker-compose pull
docker-compose build --no-cache
docker-compose up -d
```

### Zero-downtime updates

```bash
# 1. Build new image
docker-compose build app

# 2. Update with minimal downtime
docker-compose up -d app  # Blue-green with load balancer
```

---

## Support

For issues:

1. Check logs: `docker-compose logs app`
2. Verify health: `curl http://localhost:8080/health`
3. Test database: `docker-compose exec db psql -U crust_user -d crust -c "SELECT 1"`
4. Open issue: https://github.com/crust/crust/issues

---

**Last Updated**: 2026-03-05
