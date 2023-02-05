#![deny(rust_2018_idioms)]

#[cfg(test)]
mod tests;

use axum::{
    routing::{get, post},
    Extension, Router,
};
use cache::Callback;
use clap::{arg, command};
use std::{fs::remove_dir_all, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{error, info, metadata::LevelFilter};

pub mod cache;
pub mod routes;
pub mod util;

use crate::{cache::Cache, util::Config};

type TargetsCompiling = Arc<Mutex<Vec<String>>>;

#[tokio::main]
async fn main() {
    let matches = command!()
        .arg(arg!(             <repo>    "The repo to compile and distribute"))
        .arg(arg!(-t           [timeout] "How long values should live (in seconds) in the cache! Set to 0 for no cache timeout. (defaults to 1024 seconds)"))
        .arg(arg!(debug: -d --debug      "Toggled debug output"))
        .arg(arg!(--path    [path]    "The path to place \"repo_to_compile\" in. (defauls to \"./\""))
        .arg(arg!(-p --port    [port]    "The port number to host the server on (defaults to 3000"))
        .arg(arg!(-n --name    [binary_name]    "The name of the binary to return. Useful for when serving a repo which compiles multiple binaries."))
        .get_matches();

    let log_level = if matches.contains_id("debug") {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    let sub = tracing_subscriber::FmtSubscriber::builder()
        .with_level(true)
        .with_target(false)
        .with_max_level(log_level)
        .finish();

    tracing::subscriber::set_global_default(sub).unwrap();

    let port = matches.get_one::<String>("port")
        .unwrap_or(&3000.to_string())
        .parse::<u16>()
        .expect("Invalid argument!");

    if util::cross_not_found() {
        error!("the \"cross\" executable could not be found, is it installed and in path?");
        std::process::exit(0);
    } else {
        info!("Cross found!");
    }

    let mut compilation_directory: PathBuf = PathBuf::from(
        matches
            .get_one::<String>("path")
            .unwrap_or(&"./".to_string())
            .clone(),
    );
    if !compilation_directory.exists() {
        error!("The location: {compilation_directory:?} does not exist!");
        std::process::exit(0);
    } else {
        info!("Found location {compilation_directory:?}");
    }
    compilation_directory.push("repo_to_compile");

    let origin_url = matches.get_one::<String>("repo").unwrap().clone();
    info!("Pointing at repo: {origin_url}");

    // Ensure that compilation_directory exists and is empty.
    if let Err(e) = util::restore_compilation_directory(&compilation_directory) {
        error!(e);
        return;
    }

    let time_out = matches
        .get_one::<String>("timeout")
        .unwrap_or(&1024.to_string())
        .parse::<u64>()
        .expect("Invalid argument!");

    if time_out == 0 {
        info!("Cache timeout not set, data will not go out of cache.");
    } else {
        info!("Cache timeout set to {time_out} seconds.");
    }

    info!("Log level set to: {log_level}");

    // GODAHMN this is hacky
    let thing = Box::new(compilation_directory.clone());
    let dummy = Box::leak(thing.clone());
    let callback: Option<Callback> = Some(Box::new(|x| {
        let fname = dummy.join(x).into_os_string().into_string().unwrap();
        if let Err(e) = remove_dir_all(&fname) {
            info!("Callback failed to delete file: {fname} with error: {e:#?}");
        } else {
            info!("Erased \"{fname}\" from cache.");
        }
    }));

    // Create cache
    let cache = Arc::new(Mutex::new(
        Cache::new(Duration::new(time_out, 0), callback).await,
    ));

    // Create the protected vector to store the currently compiling targets in
    // with capacity since we will NEVER store more than 99 targets at the same time.
    let targets_compiling = Arc::new(Mutex::new(Vec::<String>::with_capacity(99)));

    let config = Config::new(
        matches.contains_id("debug"),
        matches.get_one::<String>("binary_name").cloned()
    );

    // build our application with some routes
    let app = Router::new()
        // Entry point for the application
        .route("/", get(routes::get_index))
        // Called by JS in index page
        .route("/get_target", post(routes::get_target))
        // Returns the actual compiled file
        .route("/get_binary/:path", get(routes::send_binary))
        .layer(Extension(cache))
        .layer(Extension(origin_url))
        .layer(Extension(compilation_directory))
        .layer(Extension(config))
        .layer(Extension(targets_compiling));

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Listening on ip: {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
