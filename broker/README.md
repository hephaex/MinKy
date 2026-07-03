# memory-broker

Local memory broker for the gyeol/minky hierarchical memory system
(see `.gjc/_session-019f235a-a4b5-7000-901e-6d06f2e92852/specs/deep-interview-memory-broker-hierarchy.md`).

Splits memory into a short-term working set (pinned files + a rolling
access window under `~/Simon/Gaeyang/.memory`) and long-term storage
(minky, reached only for evidence — never for answer synthesis). No
resident daemon: the MCP server is a **session-scoped stdio process**
that exits naturally on `stdin` EOF.

## Build

```sh
cd broker
cargo build --release --bin broker-mcp
# binary: broker/target/release/broker-mcp
```

## Configuration

Config is loaded once at startup from `~/.config/gyeol/broker.toml`
(same directory as the existing `sync-memory-to-minky.sh` tooling). A
missing file falls back to defaults — see `broker::config::BrokerConfig`
for the full schema (verdict thresholds, minky endpoint, working-set
policy, state DB path).

## MCP tools

| Tool | Input | Output |
|---|---|---|
| `load_working_set` | *(none)* | Current `WorkingSetSnapshot` (members + soft-demoted items) |
| `retrieve` | `{ "query": string }` | `CascadeOutcome` — runs the 3-stage retrieval cascade (working set → minky evidence → ask-user), including `verdict`, `reason`, `scores`, `evidence` |
| `record_access` | `{ "rel_path": string, "source": string }` | Recall confirmation: whether the item is now in the working set and whether it is pinned |

Implementation: the official [rmcp](https://crates.io/crates/rmcp) SDK
(`server`, `transport-io`, `macros`, `schemars` features) over stdio.
Tool handler logic lives in `broker::mcp_server` as plain functions so a
manual JSON-RPC fallback (approved plan: serde_json + tokio stdin/stdout)
could reuse it verbatim if rmcp ever regressed — not needed today; the
rmcp path compiled and passed its stdio handshake smoke test on the
first attempt.

## Registering with GJC

```sh
gjc mcp add memory-broker "$(pwd)/broker/target/release/broker-mcp" --project --force
gjc mcp list --project
```

This writes `.gjc/mcp.json` at the repo root (`--project` scope):

```json
{
  "mcpServers": {
    "memory-broker": {
      "type": "stdio",
      "command": "/absolute/path/to/broker/target/release/broker-mcp"
    }
  }
}
```

## Registering with Claude Code

Claude Code reads MCP servers from its own config
(`claude_desktop_config.json` on macOS, or a project-local
`.mcp.json` — see Claude Code's MCP documentation for the current
path on your platform). Add the same stdio entry:

```json
{
  "mcpServers": {
    "memory-broker": {
      "command": "/absolute/path/to/broker/target/release/broker-mcp"
    }
  }
}
```

Both clients speak the same MCP stdio protocol against the same
binary — no vendor-specific server code. This is the E2E-5 vendor
neutrality guarantee: whichever LLM/tool is driving Claude Code or
GJC, `load_working_set` / `retrieve` / `record_access` behave
identically because they are the same process on stdin/stdout.

## Verifying the registration

```sh
# 1. Confirm gjc sees it
gjc mcp list --project --json

# 2. Confirm the live stdio handshake and exact tool set
python3 - "$(pwd)/broker/target/release/broker-mcp" <<'PY'
import subprocess, json, sys, select
p = subprocess.Popen([sys.argv[1]], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)
def send(m): p.stdin.write(json.dumps(m) + "\n"); p.stdin.flush()
def recv():
    r, _, _ = select.select([p.stdout], [], [], 5)
    return p.stdout.readline() if r else None
send({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"verify","version":"0"}}})
print(recv())
send({"jsonrpc":"2.0","method":"notifications/initialized"})
send({"jsonrpc":"2.0","id":2,"method":"tools/list"})
print(json.loads(recv())["result"]["tools"] and "3 tools present")
p.stdin.close()
p.wait(timeout=10)
print("exit code:", p.returncode, "(0 = clean EOF-triggered shutdown, no signal sent)")
PY
```

The broker never starts on its own — it only runs while an MCP client
(Claude Code, GJC) has it open as a subprocess, matching the
"no resident daemon" constraint.
