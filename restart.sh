#!/usr/bin/env bash
set -e

# Change to repository root directory
cd "$(dirname "$0")"

# Check if window/desktop mode is requested
WINDOW_MODE=false
APP_MODE="local"
for arg in "$@"; do
  case $arg in
    --window|-w|--desktop|-d)
      WINDOW_MODE=true
      shift
      ;;
    --cloud)
      APP_MODE="cloud"
      shift
      ;;
  esac
done

echo "🛑 Encerrando instâncias antigas de cargo-watch, server e tauri..."
pkill -f "cargo-watch" 2>/dev/null || true
pkill -f "target/debug/server" 2>/dev/null || true
pkill -f "vibe-kanban-tauri" 2>/dev/null || true

echo "🛑 Liberando portas 3001, 3002, 3003..."
for port in 3001 3002 3003; do
  pids=$(lsof -ti :$port 2>/dev/null || true)
  if [ -n "$pids" ]; then
    echo "  -> Liberando porta $port (PID: $pids)..."
    echo "$pids" | xargs kill -9 2>/dev/null || true
  fi
done

# Limpar cache do Vite para garantir carregamento dos novos componentes visuais
rm -rf packages/local-web/node_modules/.vite node_modules/.vite target/.rustc_info.json target/debug/.cargo-lock 2>/dev/null || true

# Atualizar pipeline bundled no diretório local do usuário se existir
if [ -d "$HOME/.vibe-kanban/pipelines" ]; then
  cp assets/pipelines/swarm-multi-agent.toml "$HOME/.vibe-kanban/pipelines/swarm-multi-agent.toml" 2>/dev/null || true
fi

echo "🔄 Atualizando tipos compartilhados..."
pnpm run generate-types

if [ "$WINDOW_MODE" = true ]; then
  echo "🚀 Iniciando Vibe Kanban em MODO JANELA DESKTOP (Tauri)..."
  VIBE_KANBAN_MODE="$APP_MODE" pnpm run dev:window
else
  echo "🚀 Iniciando servidor Vibe Kanban no NAVEGADOR (Frontend :3001 | Backend :3002)..."
  VIBE_KANBAN_MODE="$APP_MODE" pnpm run dev
fi
