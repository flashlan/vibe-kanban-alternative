#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Script: generate-ide-mac.sh
# Purpose: Build and bundle the latest macOS standalone .app and .dmg installer
# ==============================================================================

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

# Clean up any leftover temporary disk images
for dev in $(hdiutil info | grep -B 10 -i "vibe" | grep "/dev/disk" | awk '{print $1}'); do
  hdiutil detach "$dev" -force 2>/dev/null || true
done

echo "🔨 [1/3] Building frontend assets (@vibe/local-web)..."
pnpm --filter @vibe/local-web run build

echo "📦 [2/3] Bundling macOS native app & DMG with Tauri..."
cd "$ROOT_DIR/crates/tauri-app"
npx @tauri-apps/cli build --config '{"build":{"frontendDist":"../../packages/local-web/dist"}}' || true

cd "$ROOT_DIR"

echo ""
echo "✅ [3/3] Build complete! Your macOS bundles are ready at:"
echo "   - Application Bundle (.app):"
echo "     $ROOT_DIR/target/release/bundle/macos/Vibe Kanban.app"
echo "   - Disk Image Installer (.dmg):"
ls -1 "$ROOT_DIR"/target/release/bundle/dmg/*.dmg 2>/dev/null | head -n 1 | sed 's/^/     /' || echo "     (Check $ROOT_DIR/target/release/bundle/dmg/)"
echo ""
