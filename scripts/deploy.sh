#!/usr/bin/env bash
# Build the production image from the current checkout and (re)start it locally.
set -euo pipefail
cd "$(dirname "$0")/.."

if ! podman machine list --format '{{.LastUp}}' | grep -q 'Currently running'; then
  echo "starting podman machine"
  podman machine start
fi

export FORGEPULSE_GIT_REF="$(git branch --show-current)@$(git rev-parse --short HEAD)"
podman build --tag forgepulse:local -f Containerfile .
podman compose -f compose.prod.yml up -d --force-recreate

for _ in $(seq 1 60); do
  if curl -sf localhost:18744/api/health; then echo; exit 0; fi
  sleep 2
done
echo "forgepulse did not become healthy" >&2
exit 1
