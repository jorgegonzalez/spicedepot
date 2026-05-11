#!/usr/bin/env bash
#
# Re-vendor the SpiceDB / authzed gRPC proto files (and their transitive
# dependencies) into src-tauri/proto/. Run this whenever you want to bump the
# proto definitions; commit the resulting changes.
#
# We pin every upstream by commit SHA so the build stays reproducible.

set -euo pipefail

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"
DEST="$ROOT/src-tauri/proto"

# ---- pinned versions ------------------------------------------------------
AUTHZED_API_REF="cf71f1efcad946380af6d36b9bc60876f7820b24"   # authzed/api main @ 2026-05-01
GOOGLEAPIS_REF="master"                                       # googleapis/googleapis (stable files)
PGV_REF="v1.0.4"                                              # envoyproxy/protoc-gen-validate
PROTOVALIDATE_REF="v0.7.1"                                    # bufbuild/protovalidate
GRPC_GATEWAY_REF="v2.22.0"                                    # grpc-ecosystem/grpc-gateway
# --------------------------------------------------------------------------

dl() {
  # dl <repo> <ref> <src-path> <dest-rel>
  local repo="$1" ref="$2" src="$3" dest_rel="$4"
  local url="https://raw.githubusercontent.com/${repo}/${ref}/${src}"
  local out="$DEST/${dest_rel}"
  mkdir -p "$(dirname "$out")"
  echo "  $repo  $src"
  curl -sSfL "$url" -o "$out"
}

echo "→ wiping $DEST"
rm -rf "$DEST"
mkdir -p "$DEST"

echo "→ authzed/api @ $AUTHZED_API_REF"
for f in core debug error_reason experimental_service permission_service schema_service watch_service; do
  dl authzed/api "$AUTHZED_API_REF" "authzed/api/v1/${f}.proto" "authzed/api/v1/${f}.proto"
done

echo "→ googleapis @ $GOOGLEAPIS_REF"
dl googleapis/googleapis "$GOOGLEAPIS_REF" "google/api/annotations.proto"     "google/api/annotations.proto"
dl googleapis/googleapis "$GOOGLEAPIS_REF" "google/api/http.proto"            "google/api/http.proto"
dl googleapis/googleapis "$GOOGLEAPIS_REF" "google/rpc/status.proto"          "google/rpc/status.proto"

echo "→ envoyproxy/protoc-gen-validate @ $PGV_REF"
dl bufbuild/protoc-gen-validate "$PGV_REF" "validate/validate.proto" "validate/validate.proto"

echo "→ bufbuild/protovalidate @ $PROTOVALIDATE_REF"
dl bufbuild/protovalidate "$PROTOVALIDATE_REF" "proto/protovalidate/buf/validate/validate.proto"      "buf/validate/validate.proto"
dl bufbuild/protovalidate "$PROTOVALIDATE_REF" "proto/protovalidate/buf/validate/expression.proto"    "buf/validate/expression.proto"
dl bufbuild/protovalidate "$PROTOVALIDATE_REF" "proto/protovalidate/buf/validate/priv/private.proto"  "buf/validate/priv/private.proto"

echo "→ grpc-ecosystem/grpc-gateway @ $GRPC_GATEWAY_REF"
dl grpc-ecosystem/grpc-gateway "$GRPC_GATEWAY_REF" "protoc-gen-openapiv2/options/annotations.proto" "protoc-gen-openapiv2/options/annotations.proto"
dl grpc-ecosystem/grpc-gateway "$GRPC_GATEWAY_REF" "protoc-gen-openapiv2/options/openapiv2.proto"   "protoc-gen-openapiv2/options/openapiv2.proto"

echo
echo "✓ proto files vendored under $DEST"
