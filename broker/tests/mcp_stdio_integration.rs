//! Integration test for the `broker-mcp` stdio server.
//!
//! Spawns the real binary as a child process, speaks newline-delimited
//! JSON-RPC 2.0 over its stdin/stdout (the MCP stdio framing), performs the
//! `initialize` handshake, and asserts that `tools/list` enumerates the three
//! broker tools. Transport-agnostic on purpose: it exercises the wire protocol,
//! not the rmcp client API.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

/// Read `stdout` lines on a background thread so the main thread can bound its
/// wait and never deadlock on a stuck child.
fn spawn_line_reader(
    stdout: std::process::ChildStdout,
) -> mpsc::Receiver<String> {
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

/// Wait for a JSON-RPC response with the given `id`, skipping notifications and
/// unrelated messages. Fails on timeout.
fn recv_response(rx: &mpsc::Receiver<String>, id: i64) -> serde_json::Value {
    let deadline = std::time::Instant::now() + Duration::from_secs(15);
    loop {
        let remaining = deadline
            .checked_duration_since(std::time::Instant::now())
            .unwrap_or_default();
        assert!(
            !remaining.is_zero(),
            "timed out waiting for JSON-RPC response id={id}"
        );
        let line = rx
            .recv_timeout(remaining)
            .unwrap_or_else(|_| panic!("timed out waiting for JSON-RPC response id={id}"));
        if line.trim().is_empty() {
            continue;
        }
        let msg: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue, // ignore any non-JSON stderr bleed (stderr is separate anyway)
        };
        if msg.get("id").and_then(|v| v.as_i64()) == Some(id) {
            return msg;
        }
    }
}

#[test]
fn stdio_server_lists_three_tools() {
    let exe = env!("CARGO_BIN_EXE_broker-mcp");

    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn broker-mcp");

    let mut stdin = child.stdin.take().expect("child stdin");
    let rx = spawn_line_reader(child.stdout.take().expect("child stdout"));

    // 1) initialize
    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"broker-it","version":"0.0.1"}}}"#;
    // 2) initialized notification (no response expected)
    let initialized = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    // 3) tools/list
    let list = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;

    writeln!(stdin, "{init}").unwrap();
    writeln!(stdin, "{initialized}").unwrap();
    writeln!(stdin, "{list}").unwrap();
    stdin.flush().unwrap();

    // initialize response
    let init_resp = recv_response(&rx, 1);
    assert!(
        init_resp.get("result").is_some(),
        "initialize should succeed: {init_resp}"
    );

    // tools/list response
    let list_resp = recv_response(&rx, 2);
    let tools = list_resp
        .pointer("/result/tools")
        .and_then(|v| v.as_array())
        .expect("tools/list result should contain a tools array");

    let names: Vec<String> = tools
        .iter()
        .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
        .collect();

    for expected in ["load_working_set", "retrieve", "record_access"] {
        assert!(
            names.iter().any(|n| n == expected),
            "tools/list should include `{expected}`; got {names:?}"
        );
    }
    assert_eq!(names.len(), 3, "exactly three tools expected; got {names:?}");

    // Closing stdin (EOF) triggers natural, daemon-free shutdown.
    drop(stdin);
    let status = wait_with_timeout(&mut child, Duration::from_secs(10));
    assert!(status, "server did not exit after stdin EOF");
}

/// Poll for child exit up to `timeout`, killing it if it overstays.
fn wait_with_timeout(child: &mut std::process::Child, timeout: Duration) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}
