# WinWeaver assistant in pi

A Podman image of the [pi agent](https://github.com/earendil-works/pi) with the WinWeaver assistant. Every build takes
the latest Ubuntu LTS, Node.js LTS and pi. pi runs with its defaults (its tools, MCP with codemode, its system prompt)
and reaches WinWeaver only through the MCP server of the program; the container holds no program, profile or answer
file. The model is yours: a local OpenAI-compatible server or a provider of pi.

| In the image | From |
|---|---|
| `/AGENTS.md` | `agents/AGENTS.md`, the instructions for any agent |
| `/work/AGENTS.md` | `agents/pi-agent/AGENTS.md`, what pi adds; pi starts in `/work` |
| `/home/pi/.agents/skills/` | `agents/skills/` |

## 1. Build and run

From the folder `agents` (the parent of this one):

```bash
podman build --pull --no-cache -t winweaver-pi -f pi-agent/Dockerfile .
podman run -it --rm --name winweaver-pi -v pi-winweaver:/home/pi/.pi --network=host winweaver-pi
```

`--no-cache` makes every build take the newest versions. The volume `pi-winweaver` keeps the settings, the model list,
the MCP entry and the sessions. A second shell: `podman exec -it winweaver-pi sh`.

## 2. First setup (once, in the second shell)

Settings, with the model you chose (`<provider>` and `<model>` as in `models.json` or a provider of pi):

```bash
cat > ~/.pi/agent/settings.json <<'EOF'
{ "enableInstallTelemetry": false, "defaultProvider": "<provider>", "defaultModel": "<model>", "defaultTools": ["+codemode"] }
EOF
```

A local server (llama.cpp, LM Studio, Ollama and others with an OpenAI-compatible API); `apiKey` must be set, any text
for a server without keys, and `contextWindow` must not exceed the context of the server:

```bash
cat > ~/.pi/agent/models.json <<'EOF'
{ "providers": { "local": { "baseUrl": "http://127.0.0.1:8080/v1", "api": "openai-completions", "apiKey": "none",
  "models": [ { "id": "<model>", "name": "<model>", "contextWindow": 32768, "maxTokens": 8192 } ] } } }
EOF
```

The MCP entry: in the WinWeaver window, MCP tab, "Start", then "Copy the client configuration"; paste it into
`~/.pi/agent/mcp.json` unchanged. Keep the name `winweaver` and no other server in this file: codemode scripts can
call every configured server.

Check: `pi mcp list` shows `winweaver` connected with 19 tools; in pi, `/mcp` shows the state and errors and
`/mcp reconnect winweaver` reconnects after a new token.

## 3. Network

`--network=host` makes `127.0.0.1` of the container the loopback of the machine that runs it, and the WinWeaver
server listens on `127.0.0.1` of the Windows PC only. With `podman machine` on Windows the container is inside WSL2:
it reaches the window only with WSL mirrored networking (`networkingMode=mirrored` in `%UserProfile%\.wslconfig`,
Windows 11 22H2 or later). That is a system setting: change it only on a machine meant for this. With
`HTTP_PROXY` set, add `NO_PROXY=127.0.0.1,localhost`.

## 4. Security

- pi's own tools reach the container, its volume and the host network. The instructions forbid using them for
  WinWeaver, but a model can ignore instructions; what protects WinWeaver is the server: the mode you choose (read at
  every start), no tool that applies settings, and no password or product key in any answer.
- The token is in `~/.pi/agent/mcp.json` in clear text. After the work press "Renew" in the MCP tab and, when the
  sessions are not needed, remove the volume: `podman volume rm pi-winweaver`.
- Run the container on a machine or virtual machine meant for it: the host network exposes every local service.
- Never use pi's `/share` or `/bug`: they upload a session.

## 5. Troubleshooting

| Symptom | Fix |
|---|---|
| The server stopped or answers with errors | `logs/mcp-<date>.log` next to `winweaver.exe` has the requests and the reason of a failure |
| Connection refused | The window is closed, the server is stopped in the MCP tab, the port differs, or the loopback is not shared (section 3) |
| 401 | The token was renewed: copy the client configuration again, then `/mcp reconnect winweaver` |
| 421 | The URL must be `http://127.0.0.1:<port>/mcp` |
| 503 | Remind the agent to call the tools one after another |
| The agent has no WinWeaver tools | `/mcp`: the state of the entry; `"defaultTools": ["+codemode"]` in `settings.json`, then `/reload` |
| No model in `/model` | `apiKey` missing or `models.json` not valid JSON |
| A context error | `contextWindow` larger than the context of the model server |
