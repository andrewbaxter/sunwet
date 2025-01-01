use {
    aargvark::{
        traits_impls::AargvarkJson,
        vark,
        Aargvark,
    },
    loga::fatal,
    native::{
        client::{
            self,
        },
        interface::config::Config,
        server,
    },
};

#[derive(Aargvark)]
enum Command {
    Query(client::QueryCommand),
    CompileQuery(client::CompileQueryCommand),
    Change(client::change::ChangeCommand),
    History(client::HistoryCommand),
    GetNode(client::GetNodeCommand),
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
        Command::CompileQuery(c) => {
            client::handle_compile_query(c)?;
        },
        Command::Change(c) => {
            client::change::handle_change(c).await?;
        },
        Command::History(c) => {
            client::handle_history(c).await?;
        },
        Command::GetNode(c) => {
            client::handle_get_node(c).await?;
        },
        Command::RunServer(config) => {
            server::main(config.value).await?;
        },
    }
    return Ok(());
}

#[tokio::main]
async fn main() {
    match main1().await {
        Ok(_) => { },
        Err(e) => {
            fatal(e);
        },
    }
}
