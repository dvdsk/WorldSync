use std::path::Path;

use tokio::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config(String);

impl Default for Config {
    fn default() -> Self {
        const DEFAULT: &str = include_str!("default.properties");
        Self(DEFAULT.to_owned())
    }
}
impl Config {
    pub fn with_port(&mut self, port: u16) -> Self {
        let start = self.0.find("server-port=").unwrap();
        let start = start + "server-port=".len();
        let stop = self.0[start..].find(char::is_whitespace).unwrap();
        let range = start..start+stop;
        self.0.replace_range(range, &port.to_string());
        self.clone()
    }
    pub async fn write(&self, dir: &Path) -> io::Result<()> {
        let mut path = dir.to_owned();
        path.push("server.properties");
        tokio::fs::write(path, &self.0).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_port() {
        let mut config = Config::default().with_port(42);
        assert_ne!(config, Config::default());
        config.with_port(25565);
        assert_eq!(config, Config::default());
    }
}


