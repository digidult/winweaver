# WinWeaver assistant in pi

The instructions of the WinWeaver assistant (`/AGENTS.md`, loaded before this file) apply in full. Where pi's general
instructions about coding, files and commands differ, the assistant's instructions win. This file adds what pi needs.

## Calling WinWeaver

The tools of the server `winweaver` are not declared to you one by one: call them from a `codemode` script, one call
at a time, and return only what you need:

```js
const status = await tools.mcp__winweaver__get_status({});
return { mode: status.mode, current: status.current };
```

Every tool takes one object (`{}` when it has no arguments). A name you are unsure of: `describeNamespace("mcp__winweaver")`.
Documents: `read_mcp_resource` with `server: "winweaver"` and a `uri` such as `winweaver://skill`.

## Skills

The skills `winweaver`, `winweaver-review`, `winweaver-change` and `winweaver-this-pc` are listed for you; load the one
that fits the task before you start it.

## pi's own tools

`read`, `bash`, `edit` and `write` reach only this container. Never use them for WinWeaver, never open `~/.pi` (it
holds the access token) and never start network requests to reach the server.
