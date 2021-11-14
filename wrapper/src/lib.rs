use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};

mod parser;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not start server process")]
    SpawnFailed(std::io::Error),
    #[error("Io error communicating with process")]
    Pipe(std::io::Error),
    #[error(
        "Server path does not exist or non-final
        component in path is not a directory"
    )]
    IncorrectServerPath,
}

pub struct Instance(Child);

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
            .map_err(Error::SpawnFailed)?;

        let stdin = wait_for(&mut child.stdin).await;
        let handle = Handle(stdin);
        Ok((Self(child), handle))
    }

    pub async fn maintain(mut self) -> Result<(), Error> {
        let mut stdout = BufReader::new(wait_for(&mut self.0.stdout).await).lines();
        let mut stderr = BufReader::new(wait_for(&mut self.0.stderr).await).lines();

        loop {
            let stop = tokio::select! {
                res = stdout.next_line() => {
                    match res {
                        Err(e) => return Err(Error::Pipe(e)),
                        Ok(Some(line)) => handle_stdout(line).await,
                        Ok(None) => false,
                    }
                }
                res = stderr.next_line() => {
                    match res {
                        Err(e) => return Err(Error::Pipe(e)),
                        Ok(Some(line)) => handle_stderr(line).await,
                        Ok(None) => false,
                    }
                }
            };
            if stop {
                return Ok(());
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

async fn handle_stdout(line: String) -> bool {
    dbg!(line);
    false
}

async fn handle_stderr(line: String) -> bool {
    dbg!(line);
    false
}

pub struct Handle(ChildStdin);

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
