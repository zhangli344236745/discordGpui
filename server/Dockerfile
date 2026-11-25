FROM python:3.13-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=ghcr.io/astral-sh/uv:latest /uv /uvx /usr/local/bin/

WORKDIR /app

COPY pyproject.toml uv.lock ./
RUN uv sync --frozen --no-dev

COPY . .

ENV PYTHONUNBUFFERED=1 \
    UPLOAD_DIR=/data/uploads \
    DATABASE_URL=postgresql+asyncpg://discord:discord@postgres:5432/discord \
    REDIS_URL=redis://redis:6379/0

RUN mkdir -p /data/uploads

EXPOSE 8080

CMD ["sh", "-c", "uv run alembic upgrade head && uv run uvicorn app.main:app --host 0.0.0.0 --port 8080"]
