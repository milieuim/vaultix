use cmd::Args;
use eyre::Result;
use simple_logger::SimpleLogger;

mod cmd;
mod util {
    pub mod callback;
    pub mod makeup;
    pub mod secbuf;
    pub mod secmap;
    pub mod set_owner_group;
}
mod parser;
mod profile;

fn main() -> Result<()> {
    SimpleLogger::new()
        .without_timestamps()
        .with_level(log::LevelFilter::Info)
        .env()
        .init()?;

    let args: Args = argh::from_env();
    args.ayaya()
}
