//! `rflap-mcp`: the Rflap MCP server. stdio by default; `--http [ADDR]` for Streamable HTTP.

use std::net::IpAddr;
use std::process::ExitCode;

use rflap_mcp::{Config, HttpOptions, HttpServer, Server, serve_stdio, spawn_viewer};

const HELP: &str = "\
rflap-mcp: Rflap as an MCP server for AI chats and agents.

USAGE:
    rflap-mcp [OPTIONS]

By default it speaks MCP over stdio (newline-delimited JSON-RPC); logs go to stderr.

OPTIONS:
    --http [ADDR]          Serve Streamable HTTP on one /mcp endpoint instead of stdio.
                           ADDR is PORT or HOST:PORT (default 127.0.0.1:8787)
    --host HOST            Host to bind with --http when ADDR is only a port (default 127.0.0.1).
                           Use 0.0.0.0 to listen on every interface
    --allow-origin ORIGIN  Also accept this Origin (repeatable, e.g. https://chatgpt.com).
                           Loopback origins are always accepted; * accepts every origin
    --viewer-port PORT     Also serve the widget at http://127.0.0.1:PORT/ so results can link to
                           it (in --http mode the same server already serves it at /)
    --no-files             stdio: disable the path arguments of rflap_open_jff / rflap_save_jff
    --allow-files          --http: enable those path arguments (off by default over HTTP,
                           because a tunnel makes a localhost server reachable from outside)
    --tools-markdown       Print the tool reference (docs/mcp-tools.md) and exit
    -h, --help             Show this help
    -V, --version          Show the version
";

struct Args {
    http: Option<String>,
    host: String,
    allow_origins: Vec<String>,
    viewer_port: Option<u16>,
    no_files: bool,
    allow_files: bool,
}

fn parse_args() -> Result<Option<Args>, String> {
    let mut args = Args {
        http: None,
        host: "127.0.0.1".into(),
        allow_origins: Vec::new(),
        viewer_port: None,
        no_files: false,
        allow_files: false,
    };
    let mut host_given = false;
    let mut rest = std::env::args().skip(1).peekable();
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(None);
            }
            "--tools-markdown" => {
                print!("{}", rflap_mcp::tools_markdown());
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("rflap-mcp {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "--http" => {
                let addr = match rest.peek() {
                    Some(next) if !next.starts_with('-') => rest.next().unwrap_or_default(),
                    _ => "8787".into(),
                };
                args.http = Some(addr);
            }
            "--host" => {
                args.host = rest.next().ok_or("--host needs a value")?;
                host_given = true;
            }
            "--allow-origin" => args
                .allow_origins
                .push(rest.next().ok_or("--allow-origin needs a value")?),
            "--viewer-port" => {
                let value = rest.next().ok_or("--viewer-port needs a value")?;
                args.viewer_port = Some(
                    value
                        .parse()
                        .map_err(|_| format!("`{value}` is not a port"))?,
                );
            }
            "--no-files" => args.no_files = true,
            "--allow-files" => args.allow_files = true,
            other => return Err(format!("unknown option `{other}`; see --help")),
        }
    }
    if let Some(addr) = &mut args.http {
        if addr.chars().all(|c| c.is_ascii_digit()) {
            *addr = format!("{}:{addr}", with_brackets(&args.host));
        } else if host_given {
            return Err("--host only applies when --http gets a bare port".into());
        }
    } else if host_given {
        return Err("--host needs --http".into());
    }
    Ok(Some(args))
}

fn with_brackets(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

fn is_local(addr: &str) -> bool {
    let host = addr
        .rsplit_once(':')
        .map_or(addr, |(h, _)| h)
        .trim_matches(['[', ']']);
    host == "localhost" || host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn main() -> ExitCode {
    // stdout belongs to the protocol: panics and diagnostics go to stderr only.
    std::panic::set_hook(Box::new(|info| eprintln!("rflap-mcp: panic: {info}")));
    let args = match parse_args() {
        Ok(Some(args)) => args,
        Ok(None) => return ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("rflap-mcp: {message}");
            return ExitCode::from(2);
        }
    };
    let mut config = Config {
        allow_files: if args.http.is_some() {
            args.allow_files
        } else {
            !args.no_files
        },
        ..Config::default()
    };
    if let Some(port) = args.viewer_port {
        match spawn_viewer(port) {
            Ok(url) => {
                eprintln!("rflap-mcp: viewer at {url}");
                config.viewer_url = Some(url);
            }
            Err(e) => eprintln!("rflap-mcp: no viewer on port {port}: {e}"),
        }
    }
    let result = match args.http {
        Some(addr) => {
            if !is_local(&addr) {
                config.allow_files = false;
                eprintln!("rflap-mcp: listening beyond localhost; file access stays disabled");
            }
            let options = HttpOptions {
                allow_origins: args.allow_origins,
            };
            HttpServer::bind(&addr, options, config).map(|server| {
                eprintln!("rflap-mcp: MCP endpoint {}", server.endpoint_url());
                if let Some(viewer) = server.viewer_url() {
                    eprintln!("rflap-mcp: viewer {viewer}");
                }
                server.run();
            })
        }
        None => serve_stdio(&Server::new(config)),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rflap-mcp: {e}");
            ExitCode::FAILURE
        }
    }
}
