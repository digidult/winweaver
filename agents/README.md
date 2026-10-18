# WinWeaver assistant for AI agents

An assistant that explains, reviews and changes WinWeaver profiles and, when you allow it, reads the settings of this
PC. It works with any AI agent whose client connects MCP servers, with a cloud or a local model. It reaches WinWeaver
only through the MCP server of the program, never through files or commands.

| File | Purpose |
|---|---|
| `AGENTS.md` | The instructions of the assistant, kept short for small context windows |
| `skills/` | Skills the agent loads when a task needs them: `winweaver` (the tools), `winweaver-review`, `winweaver-change`, `winweaver-this-pc` |
| `pi-agent/` | A ready container with the pi agent, see its `README.md` |

## 1. A folder for the assistant

Make an empty folder with nothing of WinWeaver in it: an agent that sees program files reads them instead of using the
tools. Put the instructions there under the name your client loads:

| Client | In the folder |
|---|---|
| Codex CLI, Cursor, OpenCode, pi and others that read `AGENTS.md` | `AGENTS.md` |
| Claude Code | `AGENTS.md` and a `CLAUDE.md` with the line `@AGENTS.md` |
| Gemini CLI | `AGENTS.md` copied as `GEMINI.md` |
| A chat client with a field for instructions | the text of `AGENTS.md` in that field |

Skills: copy the folders of `skills/` into the skills folder of your client, for example `.agents/skills/` of the
assistant folder (the Agent Skills standard), `.claude/skills/` for Claude Code, `~/.agents/skills/` for pi. A client
without skills reads the same reference from the server (`winweaver://skill`).

## 2. Connect the server

In the WinWeaver window, menu, "MCP server":

1. Choose the port and "Start", then "Copy the client configuration" and paste the entry into the MCP settings of your
   client under the name `winweaver`. It holds the access token: keep it out of chats and shared folders; "Renew"
   makes the old token useless.
2. A client that starts programs itself can use "Copy the stdio configuration" instead: it starts
   `winweaver.exe --mcp stdio --mode read` without the window.
3. Start the client in the assistant folder and ask "what can you do?".

The HTTP server listens on `127.0.0.1` only and refuses other addresses.

## 3. What the assistant may do

You choose the mode in the MCP tab; the assistant never changes it.

| Mode | The assistant may |
|---|---|
| read | Read the catalog and the profiles, check them, compare them, preview a build |
| edit | Also change the profiles of the server workspace in memory and create or load them |
| files | Also save profiles into `profiles/` and write the answer file into `output/` |

"Allow reading the settings of this PC" lets it read this PC (only reading, no administrator rights). The tab keeps
these settings in `mcp/default.wwmcp`; `winweaver --mcp FILE` starts the server without the window as such a file
says. The assistant never applies settings, never runs commands and never sees passwords or product keys.

The server writes what it does into `logs/mcp-<date>.jsonl` next to `winweaver.exe`, one JSON object a line (no
token, passwords or keys; the detailed log of the configuration adds every request and answer):
look there when a client loses the connection or the server stops.

## 4. Before you rely on it

The assistant asks before every change. A profile it calls valid only builds in memory: build the answer file in the
window (F9), which also checks the scripts, and install it in a virtual machine first.
