#![deny(rust_2018_idioms)]

#[cfg(test)]
mod tests;

use axum::{
    routing::{get, post},
    Extension, Router,
};
use clap::{arg, command};
use compilation_state::CompilationState;
use std::{fs::remove_dir_all, net::SocketAddr, sync::Arc, time::Duration, process};
use tokio::sync::Mutex;
use tracing::{error, info};

pub mod cache;
pub mod routes;
pub mod util;
pub mod compilation_state;


use crate::cache::Cache;

/// Represents the mutex for compilation.
type CurrentlyCompiling = Arc<Mutex<()>>;

/// Type needed to share the progress of the current compilation.
type CompilationProgress = Arc<Mutex<CompilationState>>;


#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    if util::cross_not_found() {
        error!("the \"cross\" executable could not be found, is it installed and in path?");
        std::process::exit(0);
    } else {
        info!("Cross found!");
    }

    let matches = command!()
        .arg(arg!(             [repo]    "The repo to compile and distribute"))
        .arg(arg!(-t           [timeout] "How long values should live (in seconds) in the cache! (defaults to 1024 seconds)"))
        .arg(arg!(-d --develop           "Whether to always re-pull the pointed to repository"))
        .get_matches();

    // TODO: This arg should be mandatory at release!
    let repo_name = matches
        .get_one::<String>("repo")
        .unwrap_or(&"https://github.com/Inventitech/helloworld.rs".to_string())
        .clone();
    info!("Pointing to repo: {repo_name}");

    let should_recompile = matches.contains_id("develop");
    if let Err(e) = util::ensure_repo_exists(&repo_name, should_recompile).await {
        error!(e);
        return;
    }

    let time_out = matches
        .get_one::<String>("timeout")
        .unwrap_or(&1024.to_string())
        .parse::<u64>()
        .expect("Invalid argument!");
    info!("Cache timeout set to {time_out} seconds.");


    let callback: Option<fn(String)> = Some(|x| {
        let fname = format!("binary_files/{x}");
        if let Err(e) = remove_dir_all(&fname) {
            info!("Callback failed to delete file: {fname} with error: {e:#?}");
        } else {
            info!("Erased \"{fname}\"");
        }
    });

    // Create cache
    let cache = Arc::new(Mutex::new(
        Cache::new(Duration::new(time_out, 0), callback).await,
    ));

    let currently_compiling: CurrentlyCompiling = Arc::new(Mutex::new(()));
    let compilation_progress: CompilationProgress = Arc::new(Mutex::new(CompilationState::default()));

    // build our application with some routes
    let app = Router::new()
        .route("/", get(routes::get_target))
        .route("/get_binary/:path", get(routes::send_binary))
        .route("/push", post(routes::recv).get(routes::get_target))
        .route("/status", get(routes::status))
        .layer(Extension(cache))
        .layer(Extension(repo_name))
        .layer(Extension(currently_compiling))
        .layer(Extension(compilation_progress));

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    //let addr = SocketAddr::from(([192, 168, 10, 135], 30000));
    info!("Listening on ip: {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
