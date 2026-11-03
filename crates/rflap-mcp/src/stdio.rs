//! The stdio transport: one JSON-RPC message per line on stdin, one answer per line on stdout.
//! Nothing else is ever written to stdout; diagnostics go to stderr.

use std::io::{self, BufRead, Write};

use crate::Server;

/// Serves the process's stdin/stdout until stdin closes.
pub fn serve_stdio(server: &Server) -> io::Result<()> {
    serve(server, io::stdin().lock(), io::stdout().lock())
}

/// Serves any reader/writer pair; the testable core of [`serve_stdio`].
pub fn serve<R: BufRead, W: Write>(server: &Server, mut input: R, mut output: W) -> io::Result<()> {
    let mut buffer = Vec::new();
    loop {
        buffer.clear();
        if input.read_until(b'\n', &mut buffer)? == 0 {
            return Ok(());
        }
        let line = String::from_utf8_lossy(&buffer);
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(answer) = server.handle_line(line) {
            output.write_all(answer.as_bytes())?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
}
