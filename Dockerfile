# ---- Build stage ----
FROM python:3.11-slim AS builder

ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

WORKDIR /app

# Install system dependencies for mysqlclient (MariaDB)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        gcc \
        default-libmysqlclient-dev \
        pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY requirements.txt .
RUN pip install --no-cache-dir --prefix=/install -r requirements.txt

# ---- Production stage ----
FROM python:3.11-slim

ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1

WORKDIR /app

# Install only the runtime library (no compiler)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        default-libmysqlclient-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy installed Python packages from builder
COPY --from=builder /install /usr/local

# Copy project source
COPY . .

# Create non-root user
RUN addgroup --system django && adduser --system --ingroup django django

# Create staticfiles directory with proper ownership
RUN mkdir -p /app/staticfiles && chown django:django /app/staticfiles

USER django

# Collect static files (will fail gracefully if SECRET_KEY not set at build time)
RUN python manage.py collectstatic --noinput || true

EXPOSE 8000

CMD ["gunicorn", "lista_compras.wsgi:application", "--bind", "0.0.0.0:8000", "--workers", "3"]
