
fn download_url() -> String {
    //https://www.oracle.com/java/technologies/jdk-script-friendly-urls/
    const OS: &str = "linux";
    const ARCH: &str = "x64";
    const PACK: &str = "tar.gz";
    format!("https://download.oracle.com/java/17/latest/jdk-17_{}-{}.{}", OS, ARCH, PACK)
}

pub fn download_java(dir: impl AsRef<Path>) -> Result<(), Error> {
    let url = download_url();
    let bytes = reqwest::get(url).await?.bytes().await?;


    todo!()
}
