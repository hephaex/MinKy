//! Adversarial / red-team QA for the `broker-mcp` stdio server.
//!
//! These tests spawn the *real* `broker-mcp` binary as a child process and
//! attack it over its stdin/stdout JSON-RPC wire (the MCP stdio framing). The
//! focus is robustness, not the happy path: malformed input, invalid tool
//! arguments, natural EOF shutdown (proving it is session-scoped, not a
//! resident daemon), cross-call state-DB consistency, and graceful degradation
//! when the long-term backend (minky) is unreachable.
//!
//! Every child gets its own isolated `$HOME` so config + state DB + memory root
//! are fully sandboxed and tests never touch the developer's real broker state.
//! The binary hardcodes `~/.config/gyeol/broker.toml` and `dirs::home_dir()`
//! resolves `$HOME` on this platform, so overriding `HOME` in the child env is
//! the isolation seam.

use std::io::{BufRead, BufReader, Write};

use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use filetime::{set_file_mtime, FileTime};
use tempfile::TempDir;

/// Read `stdout` lines on a background thread so the main thread can bound its
/// waits and never deadlock on a stuck child.
fn spawn_line_reader(stdout: std::process::ChildStdout) -> mpsc::Receiver<String> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    rx
}

/// A spawned `broker-mcp` child with an isolated sandbox home.
struct Server {
    child: Child,
    stdin: Option<ChildStdin>,
    rx: mpsc::Receiver<String>,
    /// Kept alive for the lifetime of the child (temp dir is deleted on drop).
    _home: TempDir,
}

impl Server {
    /// Spawn a fresh server with a sandboxed `$HOME`. Writes a broker.toml that
    /// points the memory root and state DB inside the sandbox and aims minky at
    /// a closed loopback port (`127.0.0.1:1`) so retrieval always degrades to
    /// the offline path. Also seeds a *stale* `note.md` (200 days old) used by
    /// the cross-call consistency test.
    fn spawn() -> Self {
        let home = tempfile::tempdir().expect("tempdir");
        let cfg_dir = home.path().join(".config/gyeol");
        let mem_dir = home.path().join("mem");
        std::fs::create_dir_all(&cfg_dir).expect("mkdir cfg");
        std::fs::create_dir_all(&mem_dir).expect("mkdir mem");

        let state_db = cfg_dir.join("broker.sqlite");
        let key_file = cfg_dir.join("key");
        std::fs::write(&key_file, "dummy-key\n").expect("write key");

        let toml = format!(
            "[working_set]\n\
             memory_root = {mem:?}\n\
             window_days = 30\n\
             [state_db]\n\
             path = {db:?}\n\
             [minky]\n\
             base_url = \"http://127.0.0.1:1\"\n\
             api_key_file = {key:?}\n\
             timeout_secs = 2\n",
            mem = mem_dir,
            db = state_db,
            key = key_file,
        );
        std::fs::write(cfg_dir.join("broker.toml"), toml).expect("write toml");

        // Stale memory file: mtime 200 days in the past, well outside the
        // 30-day window, so it is demoted unless a recorded access rescues it.
        let note = mem_dir.join("note.md");
        std::fs::write(&note, "# note\n").expect("write note");
        let now_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        set_file_mtime(&note, FileTime::from_unix_time(now_secs - 200 * 86_400, 0))
            .expect("set stale mtime");

        let exe = env!("CARGO_BIN_EXE_broker-mcp");
        let mut child = Command::new(exe)
            .env("HOME", home.path())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn broker-mcp");

        let stdin = child.stdin.take().expect("child stdin");
        let rx = spawn_line_reader(child.stdout.take().expect("child stdout"));
        Server {
            child,
            stdin: Some(stdin),
            rx,
            _home: home,
        }
    }

    /// Write one raw line to the child's stdin.
    fn send(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        writeln!(stdin, "{line}").expect("write stdin");
        stdin.flush().expect("flush stdin");
    }

    /// Perform the `initialize` + `notifications/initialized` handshake and
    /// assert the server accepts it.
    fn initialize(&mut self) {
        self.send(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"broker-qa","version":"0.0.1"}}}"#,
        );
        self.send(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        let resp = self.recv(1);
        assert!(
            resp.get("result").is_some(),
            "initialize must succeed: {resp}"
        );
    }

    /// Wait for a JSON-RPC message with the given `id`, skipping notifications
    /// and non-JSON bleed. Fails on timeout.
    fn recv(&self, id: i64) -> serde_json::Value {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .unwrap_or_default();
            assert!(
                !remaining.is_zero(),
                "timed out waiting for JSON-RPC message id={id}"
            );
            let line = self
                .rx
                .recv_timeout(remaining)
                .unwrap_or_else(|_| panic!("timed out waiting for JSON-RPC message id={id}"));
            if line.trim().is_empty() {
                continue;
            }
            let msg: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
                return msg;
            }
        }
    }

    /// Call a tool via `tools/call` and return the full JSON-RPC response.
    fn call_tool(&mut self, id: i64, name: &str, arguments: serde_json::Value) -> serde_json::Value {
        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments }
        });
        self.send(&req.to_string());
        self.recv(id)
    }

    /// Close stdin (EOF), signalling the client is done. Session-scoped servers
    /// must exit on their own after this — no SIGTERM/SIGKILL.
    fn close_stdin(&mut self) {
        self.stdin.take();
    }

    /// Poll for child exit up to `timeout`. Returns the exit status if it
    /// stopped in time; kills and returns `None` on overstay.
    fn wait_exit(&mut self, timeout: Duration) -> Option<std::process::ExitStatus> {
        let deadline = Instant::now() + timeout;
        loop {
            match self.child.try_wait() {
                Ok(Some(status)) => return Some(status),
                Ok(None) => {
                    if Instant::now() >= deadline {
                        let _ = self.child.kill();
                        let _ = self.child.wait();
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => return None,
            }
        }
    }

    /// True if the child is still running right now.
    fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stdin.take();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// ---------------------------------------------------------------------------
// 1. Malformed JSON-RPC / unknown methods do not crash the server.
// ---------------------------------------------------------------------------

#[test]
fn malformed_input_and_unknown_methods_do_not_crash() {
    let mut srv = Server::spawn();

    // (a) A completely broken JSON line sent *before* initialize must be
    //     silently ignored (not fatal). The subsequent valid initialize must
    //     still succeed, proving the transport survived the garbage.
    srv.send("this is not json {{{ ]][");
    srv.initialize();

    // (b) An unknown method *after* initialize must produce a JSON-RPC error
    //     response (method-not-found), and the server must keep serving.
    srv.send(r#"{"jsonrpc":"2.0","id":9,"method":"totally/unknown","params":{}}"#);
    let err_resp = srv.recv(9);
    let err = err_resp
        .get("error")
        .unwrap_or_else(|| panic!("unknown method must yield a JSON-RPC error: {err_resp}"));
    assert_eq!(
        err.get("code").and_then(|c| c.as_i64()),
        Some(-32601),
        "expected method-not-found (-32601): {err_resp}"
    );

    // Server still alive and responsive: tools/list must succeed afterwards.
    srv.send(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#);
    let list = srv.recv(2);
    let tools = list
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("tools/list must still work after garbage: {list}"));
    assert_eq!(tools.len(), 4, "four tools expected after abuse: {list}");
    assert!(srv.is_alive(), "server must remain alive after malformed input");
}

// ---------------------------------------------------------------------------
// 2. Invalid tool arguments -> graceful error result, no crash.
// ---------------------------------------------------------------------------

#[test]
fn invalid_tool_arguments_return_error_not_crash() {
    let mut srv = Server::spawn();
    srv.initialize();

    // Missing required `query` field.
    let missing = srv.call_tool(10, "retrieve", serde_json::json!({}));
    assert_eq!(
        missing.pointer("/result/isError").and_then(|v| v.as_bool()),
        Some(true),
        "missing query must be a tool error: {missing}"
    );
    let text = missing
        .pointer("/result/content/0/text")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert!(
        text.contains("query"),
        "error should mention the missing field: {missing}"
    );

    // Wrong type for `query` (integer instead of string).
    let wrong_type = srv.call_tool(11, "retrieve", serde_json::json!({ "query": 12345 }));
    assert_eq!(
        wrong_type.pointer("/result/isError").and_then(|v| v.as_bool()),
        Some(true),
        "wrong-typed query must be a tool error: {wrong_type}"
    );

    // Server survived both bad calls.
    assert!(srv.is_alive(), "server must stay alive after invalid arguments");
    let ok = srv.call_tool(12, "load_working_set", serde_json::json!({}));
    assert!(
        ok.pointer("/result/structuredContent").is_some(),
        "server must still serve valid calls: {ok}"
    );
}

// ---------------------------------------------------------------------------
// 3. EOF -> natural, daemon-free shutdown (no signals).
// ---------------------------------------------------------------------------

#[test]
fn eof_triggers_natural_shutdown_no_daemon() {
    let mut srv = Server::spawn();
    srv.initialize();

    // Confirm it is genuinely running, then close stdin with no signal at all.
    assert!(srv.is_alive(), "server should be running before EOF");
    srv.close_stdin();

    let status = srv.wait_exit(Duration::from_secs(10));
    let status = status.expect("server must self-exit within 10s of stdin EOF (not a daemon)");
    assert!(
        status.success(),
        "clean EOF shutdown should exit successfully, got {status:?}"
    );
}

// ---------------------------------------------------------------------------
// 4. record_access -> load_working_set consistency within one process.
// ---------------------------------------------------------------------------

#[test]
fn record_access_persists_across_calls_in_session() {
    let mut srv = Server::spawn();
    srv.initialize();

    // Before: note.md has a 200-day-old mtime, well outside the 30-day window,
    // so load_working_set must classify it as demoted (not a member).
    let before = srv.call_tool(20, "load_working_set", serde_json::json!({}));
    let sc = before.pointer("/result/structuredContent").expect("structuredContent");
    let member_before = sc
        .pointer("/members")
        .and_then(|m| m.as_array())
        .map(|arr| arr.iter().any(|i| i.get("rel_path").and_then(|p| p.as_str()) == Some("note.md")))
        .unwrap_or(false);
    let demoted_before = sc
        .pointer("/demoted")
        .and_then(|m| m.as_array())
        .map(|arr| arr.iter().any(|i| i.get("rel_path").and_then(|p| p.as_str()) == Some("note.md")))
        .unwrap_or(false);
    assert!(
        !member_before && demoted_before,
        "stale note.md should start demoted, not a member: {before}"
    );

    // record_access writes a fresh access timestamp into the state DB.
    let rec = srv.call_tool(
        21,
        "record_access",
        serde_json::json!({ "rel_path": "note.md", "source": "recall" }),
    );
    assert_eq!(
        rec.pointer("/result/structuredContent/recorded").and_then(|v| v.as_bool()),
        Some(true),
        "record_access must report recorded=true: {rec}"
    );

    // After: a *second, independent* tool call must observe the recorded access
    // persisted in the state DB — note.md is now a member despite its ancient
    // mtime. This proves the state DB survives across calls (no lost writes,
    // no per-call reset).
    let after = srv.call_tool(22, "load_working_set", serde_json::json!({}));
    let sc2 = after.pointer("/result/structuredContent").expect("structuredContent");
    let members = sc2.pointer("/members").and_then(|m| m.as_array()).expect("members array");
    let note = members
        .iter()
        .find(|i| i.get("rel_path").and_then(|p| p.as_str()) == Some("note.md"))
        .unwrap_or_else(|| panic!("note.md must be a member after record_access: {after}"));
    assert_eq!(
        note.get("in_working_set").and_then(|v| v.as_bool()),
        Some(true),
        "recalled note.md must be in the working set: {after}"
    );

    assert!(srv.is_alive(), "server must stay alive across the session");
}

// ---------------------------------------------------------------------------
// 5. retrieve with unreachable minky -> offline verdict, server survives.
// ---------------------------------------------------------------------------

#[test]
fn retrieve_when_minky_unreachable_degrades_offline_not_crash() {
    let mut srv = Server::spawn();
    srv.initialize();

    // minky.base_url points at 127.0.0.1:1 (closed). A query with tokens that
    // match no working-set file forces the cascade to the minky stage, which
    // fails to connect and must degrade to the offline verdict rather than
    // erroring the whole tool or killing the server.
    let resp = srv.call_tool(
        30,
        "retrieve",
        serde_json::json!({ "query": "zzz-nonexistent-token-qwxyz" }),
    );

    // The tool call itself must not be a JSON-RPC error, and the structured
    // result must carry the offline verdict.
    assert!(
        resp.get("error").is_none(),
        "retrieve must not become a JSON-RPC error when minky is down: {resp}"
    );
    let sc = resp
        .pointer("/result/structuredContent")
        .unwrap_or_else(|| panic!("retrieve must return structured content: {resp}"));
    assert_eq!(
        sc.get("verdict").and_then(|v| v.as_str()),
        Some("offline"),
        "unreachable minky must yield the offline verdict: {resp}"
    );
    assert_eq!(
        sc.get("offline").and_then(|v| v.as_bool()),
        Some(true),
        "offline flag must be set: {resp}"
    );

    // Server survived the failed backend call and still serves.
    assert!(srv.is_alive(), "server must stay alive after an offline retrieve");
    let ok = srv.call_tool(31, "load_working_set", serde_json::json!({}));
    assert!(
        ok.pointer("/result/structuredContent").is_some(),
        "server must remain responsive after offline degradation: {ok}"
    );
}