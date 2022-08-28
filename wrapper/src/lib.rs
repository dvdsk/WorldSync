use core::fmt;
use derivative::Derivative;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, instrument};

mod config;
pub mod parser;
pub use config::Config;
pub use parser::{Line, Message};

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
    #[error("Unknown error, multi line msg might be truncated: {0}")]
    Unknown(String),
    #[error("No jar file 'server.jar' found at: {0}")]
    NoJar(PathBuf),
    #[error("Java version outdated")]
    OutdatedJava { required: String },
    #[error("Eula not accepted")]
    EulaUnaccepted(&'static str),
    #[error("Unparsable output from minecraft server")]
    Unparsable(parser::Error),
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Instance {
    #[derivative(Debug = "ignore")]
    _process: Child,
    working_dir: PathBuf,
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: Lines<BufReader<ChildStderr>>,
}

const GC_ARGS: &[&str] = &[
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
        Self::start_instance(server_path.as_ref(), mem_size).await
    }

    pub async fn assert_eula_accepted(server_path: &Path) -> Result<(), Error> {
        let path = server_path.join("eula.txt");
        match tokio::fs::read_to_string(path).await {
            Ok(s) if s.contains("eula=true") => Ok(()),
            Ok(_) => Err(Error::EulaUnaccepted("present but unaccepted")),
            Err(_) => Err(Error::EulaUnaccepted("file eula.txt missing")),
        }
    }

    #[instrument(err)]
    async fn start_instance(server_path: &Path, mem_size: u8) -> Result<(Self, Handle), Error> {
        let working_dir = tokio::fs::canonicalize(server_path)
            .await
            .map_err(|_| Error::IncorrectServerPath)?;
        Self::assert_eula_accepted(server_path).await?;

        #[cfg(not(target_os = "windows"))]
        let java = server_path.join("java/bin/java");
        #[cfg(target_os = "windows")]
        let java = server_path.join("java/bin/java.exe");
        let mut child = Command::new(java)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .current_dir(&working_dir)
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
        debug!("spawned java process");

        let stdin = wait_for(&mut child.stdin).await;
        let stdout = BufReader::new(wait_for(&mut child.stdout).await).lines();
        let stderr = BufReader::new(wait_for(&mut child.stderr).await).lines();

        let mut instance = Self {
            _process: child,
            working_dir,
            stdout,
            stderr,
        };
        instance.discard_class_msg().await?;
        let handle = Handle::from(stdin);
        Ok((instance, handle))
    }

    #[instrument(err)]
    async fn discard_class_msg(&mut self) -> Result<(), Error> {
        use parser::Error::ParsingError;
        use Error::Unparsable;

        const CLASS_MSG: &str = "Starting net.minecraft.server.Main";
        match self.next_event().await {
            Err(Unparsable(ParsingError { line, .. })) if line == CLASS_MSG => Ok(()),
            Err(e) => Err(e),
            Ok(parsed) => panic!(
                "Should not be able to parse first java msg, got: {:?}",
                parsed
            ),
        }
    }

    #[instrument(err)]
    pub async fn next_event(&mut self) -> Result<parser::Line, Error> {
        loop {
            tokio::select! {
                res = self.stdout.next_line() => {
                    match res {
                        Err(e) => return Err(Error::Pipe(e.kind())),
                        Ok(Some(line)) => match parser::parse(line) {
                            Ok(line) => return Ok(line),
                            Err(e) => return Err(Error::Unparsable(e))
                        }
                        Ok(None) => continue,
                    }
                }
                res = self.stderr.next_line() => {
                    match res {
                        Err(e) => return Err(Error::Pipe(e.kind())),
                        Ok(Some(line)) => return Err(handle_stderr(line, self).await),
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

async fn collect_lines(stderr: &mut Lines<BufReader<ChildStderr>>, lines: &mut String) {
    while let Some(line) = stderr.next_line().await.unwrap() {
        lines.push('\n');
        lines.push_str(&line);
    }
}

async fn handle_stderr(line: String, instance: &mut Instance) -> Error {
    // output is probably multiline, try to collect a few more lines
    let mut lines = line;
    let collect = collect_lines(&mut instance.stderr, &mut lines);
    timeout(Duration::from_millis(10), collect).await.unwrap();

    match lines.lines().next().unwrap() {
        "Error: Unable to access jarfile server.jar" => Error::NoJar(instance.working_dir.clone()),
        "Error: LinkageError occurred while loading main class net.minecraft.bundler.Main" => {
            outdated_java_error(&lines)
        }
        _ => Error::Unknown(lines),
    }
}

#[derive(Clone)]
pub struct Handle(Arc<Mutex<ChildStdin>>);

#[derive(Debug, Hash, PartialEq, Eq, Clone, thiserror::Error)]
pub enum HandleError {
    #[error("Could not write to minecraft server")]
    Io(String),
}

impl Handle {
    pub fn from(stdin: ChildStdin) -> Self {
        Self(Arc::new(Mutex::new(stdin)))
    }
    pub async fn save(&mut self) -> Result<(), HandleError> {
        self.0
            .lock()
            .await
            .write_all(b"/save-all\n")
            .await
            .map_err(|e| e.to_string())
            .map_err(HandleError::Io)?;
        Ok(())
    }
    /// sends a message to all players in the server chat, message
    /// must be plain text
    pub async fn say(&mut self, msg: impl fmt::Display) -> Result<(), HandleError> {
        let cmd = format!("/say {}\n", msg);
        self.0
            .lock()
            .await
            .write_all(cmd.as_bytes())
            .await
            .map_err(|e| e.to_string())
            .map_err(HandleError::Io)?;
        Ok(())
    }
}

pub fn outdated_java_error(lines: &str) -> Error {
    let start = lines.find("class file version ").unwrap();
    let stop = start + lines[start..].find(')').unwrap();
    let required = lines[start..stop].to_owned();
    Error::OutdatedJava { required }
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Handle to mc server")
    }
}
