#![deny(rust_2018_idioms)]

#[cfg(test)]
mod tests;

use axum::{
    routing::{get, post},
    Extension, Router,
};
use clap::{arg, command};
use std::{fs::remove_dir_all, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{error, info, metadata::LevelFilter};

pub mod cache;
pub mod routes;
pub mod util;

use crate::cache::Cache;
use crate::util::REPO_LOCATION;

type TargetsCompiling = Arc<Mutex<Vec<String>>>;

#[tokio::main]
async fn main() {
    let matches = command!()
        .arg(arg!(             [repo]    "The repo to compile and distribute"))
        .arg(arg!(-t           [timeout] "How long values should live (in seconds) in the cache! (defaults to 1024 seconds)"))
        .arg(arg!(debug: -d --debug      "Toggled debug output"))
        .get_matches();

    let log_level = if matches.contains_id("debug") {
        LevelFilter::DEBUG
    } else {
        LevelFilter::INFO
    };

    // TODO: Still need debug calls
    let sub = tracing_subscriber::FmtSubscriber::builder()
        .with_level(true)
        .with_target(false)
        .with_max_level(log_level)
        .finish();

    tracing::subscriber::set_global_default(sub).unwrap();


    //tracing_subscriber::fmt::init();

    if util::cross_not_found() {
        error!("the \"cross\" executable could not be found, is it installed and in path?");
        std::process::exit(0);
    } else {
        info!("Cross found!");
    }


    // TODO: This arg should be mandatory at release!
    let repo_name = matches
        .get_one::<String>("repo")
        .unwrap_or(&"https://github.com/Inventitech/helloworld.rs".to_string())
        .clone();
    info!("Pointing at repo: {repo_name}");

    // Ensure that REPO_LOCATION exists and is empty.
    if let Err(e) = util::restore_repo_location() {
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
        let fname = format!("{REPO_LOCATION}/{x}");
        if let Err(e) = remove_dir_all(&fname) {
            info!("Callback failed to delete file: {fname} with error: {e:#?}");
        } else {
            info!("Erased \"{fname}\" from cache.");
        }
    });

    // Create cache
    let cache = Arc::new(Mutex::new(
        Cache::new(Duration::new(time_out, 0), callback).await,
    ));

    // Create the protected vector to store the currently compiling targets in
    // with capacity since we will NEVER store more than 99 targets at the same time.
    let targets_compiling = Arc::new(Mutex::new(Vec::<String>::with_capacity(99)));

    // build our application with some routes
    let app = Router::new()
        .route("/", get(routes::get_target))
        .route("/get_binary/:path", get(routes::send_binary))
        .route("/push", post(routes::recv).get(routes::get_target))
        .route("/status", get(routes::status))
        .layer(Extension(cache))
        .layer(Extension(repo_name))
        .layer(Extension(targets_compiling));

    // run it
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    //let addr = SocketAddr::from(([192, 168, 10, 135], 30000));
    info!("Listening on ip: {addr}");
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
