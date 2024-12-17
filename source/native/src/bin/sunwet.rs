use {
    aargvark::{
        traits_impls::AargvarkJson,
        vark,
    },
    native::{
        client::{
            self,
            QueryCommand,
        },
        interface::config::Config,
    },
};

#[derive(Aargvark)]
enum Command {
    Query(client::QueryCommand),
    Change(client::ChangeCommand),
    History(client::HistoryCommand),
    RunServer(AargvarkJson<Config>),
}

#[derive(Aargvark)]
struct Args {
    command: Command,
}

async fn main1() -> Result<(), loga::Error> {
    let args = vark::<Args>();
    match args.command {
        Command::Query(c) => {
            client::handle_query(c).await?;
        },
        Command::Change(c) => {
            client::handle_change(c).await?;
        },
        Command::History(c) => {
            client::handle_history(c).await?;
        },
        Command::RunServer(config) => {
            serverlib::main(config).await?;
        },
    }
    return Ok(());
}

#[tokio::main]
async fn main() {
    match inner().await {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
}
