#!/usr/bin/env bash

RUNNING_CONTAINER=$(podman ps --filter 'name=redis' --format '{{.ID}}')

if [[ -n $RUNNING_CONTAINER ]]; then
  echo >&2 "there is a redis container already running, kill it with:"
  echo >&2 "    podman kill ${RUNNING_CONTAINER}"
  exit 1
fi

# Launch Redis using podman

podman run \
  -p "6379:6379" \
  -d \
  --name "redis_$(date '+%s')" \
  redis:7

>&2 echo "Redis is ready to go!"
