FROM node:22-bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git python3 \
    && rm -rf /var/lib/apt/lists/*

RUN npm install --global @openai/codex@0.144.0

COPY container-entrypoint.sh /usr/local/bin/benchmark-entrypoint
RUN chmod 0755 /usr/local/bin/benchmark-entrypoint

ENTRYPOINT ["/usr/local/bin/benchmark-entrypoint"]
CMD ["python3", "/benchmarks/smoke.py"]
