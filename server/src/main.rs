use server::{Sessions, db::user::UserDb};
use typed_sled::sled;
use structopt::StructOpt;
mod admin_ui;

#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(long, default_value = "8080")]
    port: u16,
    #[structopt(long)]
    admin_ui: bool
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    if opt.admin_ui {
        admin_ui::run(opt.port).await;
        return 
    }

    let subscriber = tracing_subscriber::fmt()
        .finish();

    let db = sled::open("db").unwrap();
    let sessions = Sessions::new();
    let user_db = UserDb::open(db);
    server::host(sessions, user_db, opt.port).await;
}
