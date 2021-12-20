use vergen::{Config, vergen, TimestampKind, ShaKind};

fn main() {
    // Generate the default 'cargo:' instruction output
    let mut config = Config::default();
    let build = config.build_mut();
    *build.enabled_mut() = true;
    *build.timestamp_mut() = true;
    *build.kind_mut() = TimestampKind::DateOnly;
    let git = config.git_mut();
    *git.enabled_mut() = true;
    *git.sha_mut() = true;
    *git.sha_kind_mut() = ShaKind::Short;

    vergen(config).unwrap();
}
