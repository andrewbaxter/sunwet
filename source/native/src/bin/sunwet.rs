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
#[vark(break_help)]
enum Command {
    /// Send a query (JSON) to the API and write the results (JSON) to stdout.
    Query(client::QueryCommand),
    /// Compile a query into JSON to use in config or the API.
    ///
    /// Specify a query either inline on the command line, or as a file.
    CompileQuery(client::CompileQueryCommand),
    /// Compile a query head (root and steps) into JSON, to be combined with a tail
    /// before using in config or the API.
    ///
    /// Specify a query either inline on the command line, or as a file.
    CompileQueryHead(client::CompileQueryHeadCommand),
    /// Compile a query tail (selection) into JSON, to be combined with a head before
    /// using in config or the API.
    ///
    /// Specify a query either inline on the command line, or as a file.
    CompileQueryTail(client::CompileQueryTailCommand),
    /// Add data and files to the database.
    ///
    /// This takes a CLI commit JSON, prepares and sends an API JSON commit payload
    /// (replacing files with hashes), then uploads all files.
    Commit(client::commit::CommitCommand),
    /// Prepare a CLI commit JSON from media files in a directory using their tags.
    PrepareMediaImportCommit(client::media_import::PrepareImportCommitCommand),
    /// Move all triples centered around one node to another node, eliminating the
    /// first node.
    MergeNodes(client::MergeNodesCommand),
    /// Delete all triples centered around one node.
    DeleteNodes(client::DeleteNodesCommand),
    /// Show commit history.
    History(client::HistoryCommand),
    /// Show all triples involving a given node.
    GetNode(client::GetNodeCommand),
    /// Run the Sunwet server.
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
        Command::CompileQueryHead(c) => {
            client::handle_compile_query_head(c)?;
        },
        Command::CompileQueryTail(c) => {
            client::handle_compile_query_tail(c)?;
        },
        Command::Commit(c) => {
            client::commit::handle_commit(c).await?;
        },
        Command::PrepareMediaImportCommit(c) => {
            client::media_import::handle_prepare_media_import_commit(c).await?;
        },
        Command::DeleteNodes(c) => {
            client::handle_delete_nodes(c).await?;
        },
        Command::MergeNodes(c) => {
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
