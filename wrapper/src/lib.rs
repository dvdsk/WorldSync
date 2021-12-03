use std::io;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};

pub mod parser;

#[derive(Clone, Debug, thiserror::Error, Hash, PartialEq, Eq)]
pub enum Error {
    #[error("Could not start server process")]
    SpawnFailed(io::ErrorKind),
    #[error("Io error communicating with process")]
    Pipe(io::ErrorKind),
    #[error(
        "Server path does not exist or non-final
        component in path is not a directory"
    )]
    IncorrectServerPath,
    #[error("Could not parse minecraft server output: {0}")]
    Parser(#[from] parser::Error),
}

pub struct Instance {
    process: Child,
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Lines<BufReader<ChildStderr>>,
}

const GC_ARGS: &[&'static str] = &[
    "-Dsun.rmi.dgc.server.gcInterval=2147483646", // do not garbace collect every min
    "-XX:+UnlockExperimentalVMOptions",           // unknown but recommanded
    "-XX:G1NewSizePercent=20",                    // G1GC keep 20% of heap for new objects
    "-XX:G1ReservePercent=20",                    // --
    "-XX:MaxGCPauseMillis=50",                    // try to keep GC to 50 ms
    "-XX:G1HeapRegionSize=32M",                   // allocale in blocks of 32megs
];

impl Instance {
    /// this assumes the server jar is named `server.jar` and located in
    /// the folder passed as paramater `server_path`
    pub async fn start(
        server_path: impl AsRef<Path>,
        mem_size: u8,
    ) -> Result<(Self, Handle), Error> {
        let full_path = tokio::fs::canonicalize(server_path)
            .await
            .map_err(|_| Error::IncorrectServerPath)?;
        let mut child = Command::new("java")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .current_dir(full_path)
            .arg("-XX:+UseG1GC") // use G1GC garbace collector
            // max memory size, recommanded 4G
            .arg(format!("-Xmx{}G", mem_size))
            // min size must equal max to keep GC calm
            .arg(format!("-Xms{}G", mem_size))
            .args(GC_ARGS)
            .args(["-jar", "server.jar", "nogui"])
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| Error::SpawnFailed(e.kind()))?;

        let stdin = wait_for(&mut child.stdin).await;
        let stdout = BufReader::new(wait_for(&mut child.stdout).await).lines();
        let stderr = BufReader::new(wait_for(&mut child.stderr).await).lines();

        let instance = Self {
            process: child,
            stdout,
            stderr,
        };
        let handle = Handle(stdin);
        Ok((instance, handle))
    }

    pub async fn next_event(&mut self) -> Result<parser::Line, Error> {
        loop {
            tokio::select! {
                res = self.stdout.next_line() => {
                    match res {
                        Err(e) => return Err(Error::Pipe(e.kind())),
                        Ok(Some(line)) => return handle_stdout(line),
                        Ok(None) => continue,
                    }
                }
                res = self.stderr.next_line() => {
                    match res {
                        Err(e) => return Err(Error::Pipe(e.kind())),
                        Ok(Some(line)) => return Err(handle_stderr(line)),
                        Ok(None) => continue,
                    }
                }
            }
        }
    }
}

async fn wait_for<T>(source: &mut Option<T>) -> T {
    use tokio::time::sleep;
    loop {
        if let Some(stdin) = source.take() {
            break stdin;
        }
        sleep(Duration::from_millis(50)).await;
    }
}

fn handle_stdout(line: String) -> Result<parser::Line, Error> {
    parser::parse(line).map_err(|e| e.into())
}

fn handle_stderr(line: String) -> Error  {
    todo!()
}

pub struct Handle(ChildStdin);

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Handle to mc server")
    }
}
