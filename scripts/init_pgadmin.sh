#!/usr/bin/bash

podman run --name newsletter-pgadmin \
  --network newsletter-network \
  -e PGADMIN_DEFAULT_EMAIL="admin@admin.com" \
  -e PGADMIN_DEFAULT_PASSWORD="admin" \
  -p 8080:80 \
  -d elestio/pgadmin:REL-9_11
