FROM python:3.14-slim
RUN pip install --no-cache-dir Pillow==13.0.0
ENV PYTHONDONTWRITEBYTECODE=1
WORKDIR /work
