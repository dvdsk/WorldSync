use std::path::Path;
use crate::Config;

pub async fn setup_server(dir: &Path, port: u16) {
    const URL: &str = "https://launcher.mojang.com/v1/objects\
    /3cf24a8694aca6267883b17d934efacc5e44440d/server.jar";
    let response = reqwest::get(URL).await.unwrap();

    tokio::fs::create_dir_all(dir).await.unwrap();
    let mut jar_path = dir.to_owned();
    jar_path.push("server.jar");
    let mut eula_path = dir.to_owned();
    eula_path.push("eula.txt");
    let bytes = response.bytes().await.unwrap();
    tokio::fs::write(jar_path, bytes).await.unwrap();
    tokio::fs::write(eula_path, "eula=true").await.unwrap();
    Config::default().with_port(port).write(dir).await.unwrap();
}
