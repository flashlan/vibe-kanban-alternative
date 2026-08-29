#!/usr/bin/env bash
set -euo pipefail

# Freeze and restore deployment configuration only. Application source code,
# databases, repositories, and worktrees are intentionally out of scope.

readonly STATE_DIR="${DEMO_CONFIG_STATE_DIR:-/var/lib/aurapunk-demo/config-baselines}"
readonly ACTIVE_FILE="$STATE_DIR/active"
readonly CONFIG_FILES=(
  "/etc/systemd/system/vibe-kanban-demo.service"
  "/etc/systemd/system/aurapunk-demo-server.service"
  "/etc/systemd/system/aurapunk-website.service"
  "/etc/systemd/system/aura-gateway.service"
  "/home/aurapunk/.config/opencode/opencode.jsonc"
  "/home/aurapunk/demo-server/opencode.jsonc"
  "/home/aurapunk/demo-server/.zed/settings.json"
)

usage() {
  cat <<'EOF'
Usage:
  demo-config-lock.sh freeze <name>
  demo-config-lock.sh reset [name]
  demo-config-lock.sh unfreeze [name]
  demo-config-lock.sh status

freeze  Save the current deployment configuration under <name> and activate it.
reset   Restore the active (or named) snapshot; backs up current config first.
unfreeze Remove the active marker; the snapshot remains available for rollback.
status  Show the active snapshot and the tracked configuration files.
EOF
}

require_root() {
  if [[ "$(id -u)" -ne 0 ]]; then
    echo "Run this script as root; it changes system service configuration." >&2
    exit 1
  fi
}

validate_name() {
  local name="$1"
  if [[ ! "$name" =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ ]]; then
    echo "Invalid snapshot name: $name" >&2
    exit 1
  fi
}

snapshot_dir() {
  printf '%s/%s' "$STATE_DIR" "$1"
}

copy_config_into() {
  local destination="$1"
  mkdir -p "$destination/files"

  local file relative
  for file in "${CONFIG_FILES[@]}"; do
    if [[ ! -f "$file" ]]; then
      echo "Missing tracked configuration: $file" >&2
      exit 1
    fi
    relative="${file#/}"
    mkdir -p "$destination/files/$(dirname "$relative")"
    cp --preserve=mode,ownership,timestamps "$file" "$destination/files/$relative"
  done

  date -u +%Y-%m-%dT%H:%M:%SZ > "$destination/created-at"
  printf '%s\n' "${CONFIG_FILES[@]}" > "$destination/manifest"
}

freeze() {
  local name="$1"
  validate_name "$name"
  mkdir -p "$STATE_DIR"
  local destination
  destination="$(snapshot_dir "$name")"
  if [[ -e "$destination" ]]; then
    echo "Snapshot already exists: $name (choose another name)" >&2
    exit 1
  fi
  copy_config_into "$destination"
  printf '%s\n' "$name" > "$ACTIVE_FILE"
  echo "Configuration frozen at: $name"
}

resolve_name() {
  local requested="${1:-}"
  if [[ -n "$requested" ]]; then
    printf '%s' "$requested"
    return
  fi
  if [[ ! -s "$ACTIVE_FILE" ]]; then
    echo "No active configuration snapshot." >&2
    exit 1
  fi
  tr -d '\n' < "$ACTIVE_FILE"
}

backup_current() {
  local backup_root="$STATE_DIR/backups/$(date -u +%Y%m%dT%H%M%SZ)"
  copy_config_into "$backup_root"
  echo "Current configuration backed up at: $backup_root"
}

reset_config() {
  local name
  name="$(resolve_name "${1:-}")"
  validate_name "$name"
  local source
  source="$(snapshot_dir "$name")/files"
  if [[ ! -d "$source" ]]; then
    echo "Snapshot not found: $name" >&2
    exit 1
  fi

  backup_current

  local file relative
  while IFS= read -r file; do
    relative="${file#/}"
    cp --preserve=mode,ownership,timestamps "$source/$relative" "/$relative"
  done < "$(snapshot_dir "$name")/manifest"

  printf '%s\n' "$name" > "$ACTIVE_FILE"
  systemctl daemon-reload
  systemctl try-restart vibe-kanban-demo.service aurapunk-demo-server.service 2>/dev/null || true
  echo "Configuration reset to: $name"
}

unfreeze() {
  local name
  name="$(resolve_name "${1:-}")"
  validate_name "$name"
  [[ -d "$(snapshot_dir "$name")" ]] || {
    echo "Snapshot not found: $name" >&2
    exit 1
  }
  if [[ -f "$ACTIVE_FILE" ]] && [[ "$(tr -d '\n' < "$ACTIVE_FILE")" == "$name" ]]; then
    rm -f "$ACTIVE_FILE"
  fi
  echo "Configuration unfrozen: $name (snapshot retained)"
}

status() {
  if [[ -s "$ACTIVE_FILE" ]]; then
    echo "Active snapshot: $(tr -d '\n' < "$ACTIVE_FILE")"
  else
    echo "Active snapshot: none"
  fi
  echo "Tracked configuration:"
  printf '  %s\n' "${CONFIG_FILES[@]}"
  if [[ -d "$STATE_DIR" ]]; then
    echo "Available snapshots:"
    find "$STATE_DIR" -mindepth 1 -maxdepth 1 -type d -printf '  %f\n' | sort
  fi
}

main() {
  local action="${1:-}"
  case "$action" in
    freeze)
      require_root
      [[ $# -eq 2 ]] || { usage >&2; exit 2; }
      freeze "$2"
      ;;
    reset)
      require_root
      [[ $# -le 2 ]] || { usage >&2; exit 2; }
      reset_config "${2:-}"
      ;;
    unfreeze)
      require_root
      [[ $# -le 2 ]] || { usage >&2; exit 2; }
      unfreeze "${2:-}"
      ;;
    status)
      status
      ;;
    *)
      usage >&2
      exit 2
      ;;
  esac
}

main "$@"
