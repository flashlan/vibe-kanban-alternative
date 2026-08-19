#!/usr/bin/env bash
set -e

# Change to repository root directory
cd "$(dirname "$0")"

echo "🛑 Encerrando instâncias antigas de cargo-watch e server..."
pkill -f "cargo-watch" 2>/dev/null || true
pkill -f "target/debug/server" 2>/dev/null || true

echo "🛑 Liberando portas 3001, 3002, 3003..."
for port in 3001 3002 3003; do
  pids=$(lsof -ti :$port 2>/dev/null || true)
  if [ -n "$pids" ]; then
    echo "  -> Liberando porta $port (PID: $pids)..."
    echo "$pids" | xargs kill -9 2>/dev/null || true
  fi
done

# Limpar possíveis travas órfãs do cargo no target
rm -f target/.rustc_info.json target/debug/.cargo-lock 2>/dev/null || true

echo "🔄 Atualizando tipos compartilhados..."
pnpm run generate-types

echo "🚀 Iniciando servidor Vibe Kanban (Frontend :3001 | Backend :3002)..."
pnpm run dev
