use {
    aargvark::{
        vark,
        Aargvark,
    },
    loga::fatal,
    native::{
        client::{
            self,
        },
        server,
    },
};

#[derive(Aargvark)]
enum Command {
    Query(client::QueryCommand),
    CompileQuery(client::CompileQueryCommand),
    Commit(client::commit::CommitCommand),
    PrepareImportCommit(client::import_::PrepareImportCommitCommand),
    PrepareMergeCommit(client::MergeNodesCommand),
    PrepareDeleteCommit(client::DeleteNodesCommand),
    History(client::HistoryCommand),
    GetNode(client::GetNodeCommand),
    RunServer(server::Args),
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
        Command::Commit(c) => {
            client::commit::handle_commit(c).await?;
        },
        Command::PrepareImportCommit(c) => {
            client::import_::handle_prepare_import_commit(c).await?;
        },
        Command::PrepareDeleteCommit(c) => {
            client::handle_delete_nodes(c).await?;
        },
        Command::PrepareMergeCommit(c) => {
            client::handle_merge_nodes_command(c).await?;
        },
        Command::History(c) => {
            client::handle_history(c).await?;
        },
        Command::GetNode(c) => {
            client::handle_get_node(c).await?;
        },
        Command::RunServer(config) => {
            server::main(config).await?;
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
