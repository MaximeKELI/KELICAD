#!/bin/sh
# Stub worker: import mode unwraps KELICAD_DWG_PROXY1 → DXF on stdout;
# export mode wraps stdin DXF as proxy DWG. Not a real ODA converter.
set -e
MODE="${KELICAD_DWG_MODE:-import}"
MAGIC="KELICAD_DWG_PROXY1"
if [ "$MODE" = "import" ]; then
  # strip first line magic if present
  IFS= read -r first || true
  rest=$(cat)
  case "$first" in
    "$MAGIC") printf '%s' "$rest" ;;
    *) printf '%s\n%s' "$first" "$rest" ;;
  esac
else
  printf '%s\n' "$MAGIC"
  cat
fi
