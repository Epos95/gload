#![deny(rust_2018_idioms)]

#[cfg(test)]
mod tests;

use axum::{
    routing::{get, post},
    Extension, Router,
};
use cache::Callback;
use clap::{arg, command};
use std::{fs::remove_dir_all, net::SocketAddr, sync::Arc, time::Duration, path::PathBuf};
use tokio::sync::Mutex;
use tracing::{error, info, metadata::LevelFilter};

pub mod cache;
pub mod routes;
pub mod util;

use crate::cache::Cache;

type TargetsCompiling = Arc<Mutex<Vec<String>>>;

#[tokio::main]
async fn main() {
    let matches = command!()
        .arg(arg!(             <repo>    "The repo to compile and distribute"))
        .arg(arg!(-t           [timeout] "How long values should live (in seconds) in the cache! Set to 0 for no cache timeout. (defaults to 1024 seconds)"))
        .arg(arg!(debug: -d --debug      "Toggled debug output"))
        .arg(arg!(-p --path [path] "The path to place \"repo_to_compile\" in. (defauls to \"./repo_to_compile\""))
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

    if util::cross_not_found() {
        error!("the \"cross\" executable could not be found, is it installed and in path?");
        std::process::exit(0);
    } else {
        info!("Cross found!");
    }

    let mut repo_location: PathBuf = PathBuf::from(
        matches.get_one::<String>("path")
        .unwrap_or(&"./".to_string())
        .clone());
    if !repo_location.exists() {
        error!("The location: {repo_location:?} does not exist!");
        std::process::exit(0);
    } else {
        info!("Found location {repo_location:?}");
    }
    repo_location.push("repo_to_compile");


    let repo_name = matches
        .get_one::<String>("repo")
        .unwrap()
        .clone();
    info!("Pointing at repo: {repo_name}");

    // Ensure that REPO_LOCATION exists and is empty.
    if let Err(e) = util::restore_repo_location(&repo_location) {
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

    /*
    let callback: Option<fn(String)> = Some(|x| {
        let fname = format!("{REPO_LOCATION}/{x}");
        if let Err(e) = remove_dir_all(&fname) {
            info!("Callback failed to delete file: {fname} with error: {e:#?}");
        } else {
            info!("Erased \"{fname}\" from cache.");
        }
    });
    */

    // GODAHMN this is hacky
    let thing = Box::new(repo_location.clone());
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

    // build our application with some routes
    let app = Router::new()
        // Entry point for the application
        .route("/", get(routes::get_index))
        // Called by JS in index page
        .route("/get_target", post(routes::get_target))
        // Returns the actual compiled file
        .route("/get_binary/:path", get(routes::send_binary))

        //.route("/push", post(routes::recv).get(routes::get_target))
        .route("/status", get(routes::status))
        .layer(Extension(cache))
        .layer(Extension(repo_name))
        .layer(Extension(repo_location))
        .layer(Extension(matches.contains_id("debug")))
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
