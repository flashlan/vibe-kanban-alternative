# AuraPunk Demo Server

Small dependency-free HTTP service used as the first repository in the
Vibe Kanban demo workspace.

## Run

```bash
npm start
```

The server listens on `PORT` (default `4000`) and exposes:

- `GET /` — service metadata
- `GET /health` — health check

It binds to `127.0.0.1` by default. Set `HOST=0.0.0.0` only when the demo
must be reachable from another machine.

The Vibe Kanban repository is configured with `PORT=4000 npm start` for
workspace previews. The optional systemd smoke-test service uses port `4100`
so it does not occupy the workspace preview port.

## Demo deployment on the Debian LXC

The demo is intentionally native to the container; it does not run Docker
inside the LXC.

| Item | Value |
| --- | --- |
| Container | `192.168.1.168` (`mem0-server`) |
| Linux user | `aurapunk` (never run the app as `root`) |
| Repository | `/home/aurapunk/demo-server` |
| Branch | `main` |
| Vibe Kanban project | `AuraPunk Demo` |
| Workspace command | `PORT=4000 npm start` |
| Smoke-test service | `aurapunk-demo-server.service` on `127.0.0.1:4100` |
| Public gateway | `http://192.168.1.168:3000` |
| Website | `http://192.168.1.168:3000/` |
| Vibe Kanban demo | `http://192.168.1.168:3000/demo/` |

Connect with:

```bash
ssh aurapunk-demo
cd /home/aurapunk/demo-server
```

Open the same path in Zed using remote development:

```text
ssh://aurapunk-demo/home/aurapunk/demo-server
```

OpenCode runs from the same directory as `aurapunk`. Its project settings
are in `opencode.jsonc`; Zed settings are in `.zed/settings.json`.

## Update a demo release

Run the following inside the container as `aurapunk` after placing the new
repository files in `/home/aurapunk/demo-server`:

```bash
cd /home/aurapunk/demo-server
git status
node --check server.js
npm test
git add .
git commit -m "chore: update demo release"
curl -fsS http://127.0.0.1:4100/health
```

Restart the system service from the container's administrative shell:

```bash
systemctl restart aurapunk-demo-server.service
```

For a transfer from the development machine, copy only the release contents
and preserve ownership:

```bash
scp -r deploy/demo-server/. aurapunk-demo:/home/aurapunk/demo-server/
ssh aurapunk-demo 'cd /home/aurapunk/demo-server && npm test && curl -fsS http://127.0.0.1:4100/health'
ssh -i ~/.ssh/vk-miranda-192-168-1-108 root@192.168.1.168 'chown -R aurapunk:aurapunk /home/aurapunk/demo-server && systemctl restart aurapunk-demo-server.service'
```

The frozen demo code baseline is the revision tagged `demo-baseline`. The
active deployment-configuration baseline is named separately by
`demo-config-lock.sh`. Daily reset must replace only the deployment
configuration from that baseline copy; do not rebuild or modify application
code as part of that reset. If a full demo checkout rollback is explicitly
required, use the protected tag:

```bash
git status --short
git reset --hard demo-baseline
git clean -fd
```

The last two commands intentionally remove uncommitted demo files. Confirm
the target path before running them.

## Freeze and daily reset of configuration

Deployment configuration is managed separately from application code. The
configuration lock script tracks the two systemd units and the OpenCode/Zed
project settings. It does not reset the repository, database, worktrees, or
agent changes.

Install the script in the container from the development machine and create
the current baseline:

```bash
scp deploy/demo-config-lock.sh root@192.168.1.168:/tmp/demo-config-lock.sh
ssh -i ~/.ssh/vk-miranda-192-168-1-108 root@192.168.1.168 'install -o root -g root -m 755 /tmp/demo-config-lock.sh /usr/local/sbin/demo-config-lock.sh && rm -f /tmp/demo-config-lock.sh'
demo-config-lock.sh freeze baseline-2026-08-29
```

Daily operations:

```bash
demo-config-lock.sh status
demo-config-lock.sh reset
```

To update the configuration to a new intentional point, edit the relevant
configuration files, test the services, then create a new named snapshot:

```bash
demo-config-lock.sh freeze config-2026-08-30
```

To temporarily release the lock without deleting its recovery point:

```bash
demo-config-lock.sh unfreeze config-2026-08-30
```

Every reset first stores a timestamped backup under
`/var/lib/aurapunk-demo/config-baselines/backups/`. Only the configuration
files listed by `demo-config-lock.sh` are replaced.

This repository is intentionally small so an agent can make a visible change
quickly without downloading dependencies.
