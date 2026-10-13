FROM python:3.12.10-slim

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

WORKDIR /app
COPY publier.py /app/publier.py

ENTRYPOINT ["python", "/app/publier.py"]
