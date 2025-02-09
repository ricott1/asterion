use asterion::{ssh::AppServer, store_path, AppResult};
use clap::{ArgAction, Parser};
use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};

const DEFAULT_PORT: u16 = 2020;

#[derive(Parser, Debug)]
#[clap(name="Asterion", about = "Find your way in da maze", author, version, long_about = None)]
struct Args {
    #[clap(long, short = 'p', action=ArgAction::Set, help = "Set port to listen on")]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> AppResult<()> {
    let logfile_path = store_path("minotaur.log")?;
    let logfile = FileAppender::builder()
        .append(false)
        .encoder(Box::new(PatternEncoder::new("{l} - {m}\n")))
        .build(logfile_path)?;

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(LevelFilter::Info))?;

    log4rs::init_config(config)?;

    let port = Args::parse().port.unwrap_or(DEFAULT_PORT);
    let mut game_server = AppServer::new(port);
    game_server.run().await?;

    Ok(())
}
