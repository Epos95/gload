use axum::{
    body::{self, Full},
    extract::Path,
    response::{IntoResponse, Response},
    Extension, Json,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, fs::{OpenOptions, self}, time::Duration};
use tokio::{fs::File, io::AsyncReadExt, sync::Mutex, time::sleep};
use tracing::{error, info, debug};

use crate::cache::Cache ;
use crate::util;
use crate::TargetsCompiling;

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct PostData {
    os: String,
    os_version: String,
    user_agent: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct ResponseData {
    target_triple: String
}

#[derive(Debug, Deserialize, Serialize)]
struct PostDataHolder{ post_data: Vec<PostData> }


pub async fn get_index() -> impl IntoResponse {
    // Send the html of the page which gets the target triple
    let mut file = File::open("templates/index.html").await.unwrap();
    let mut html = String::new();
    file.read_to_string(&mut html).await.unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .body(body::boxed(Full::from(html)))
        .unwrap()
}

pub async fn get_target(Json(json): Json<PostData>) -> Result<impl IntoResponse, String> {
    debug!("Recieved {json:?} on get_target");
    // Use this pattern so that we can tweak each part of the target_triple
    // individually for weird edge cases and things.

    let architecture = if json.user_agent.contains("x64") || json.user_agent.contains("x86_64") || json.os == "Mac OS X" {
        "x86_64"
    } else {
        debug!("32bit architecture detected, might give errors");
        debug!("json in question: {json:?}");
        "i686"
    };

    // Assume 64 bit computer
    let middle = match json.os.as_str() {
        "Mac OS X" => "apple",
        "Windows" => "pc-windows",
        "Linux" => "unknown-linux",
        _ => {
            error!("Failed to match middlepart for: {}", json.os);
            return Err("Sorry! We failed to compute target triple for your pc!".to_string());
        }
    };

    let toolchain = match json.os.as_str() {
        "Mac OS X" => "darwin",
        "Windows" => "gnu",
        "Linux" => "gnu",
        _ => {
            error!("Failed to match toolchain for: {}", json.os);
            return Err("Sorry! We failed to compute target triple for your pc!".to_string());
        }
    };


    let target_triple = vec![architecture, middle, toolchain].join("-");
    info!("Guessed target_triple: {target_triple}");

    Ok(Json(ResponseData { target_triple }))
    // Return a ResponseData
}


// From the data sent here the server should respond with a token for which specific file to download
// the token can then be used by GETting a route with the token as a parameter or something :D
// This token could probably be the target_triple, which would prolly work out nicely.
pub async fn recv(Json(json): Json<PostData>) -> impl IntoResponse {
    // maybe write
    info!("Recvd: {json:#?}");

    let data = fs::read_to_string("data.json").unwrap();
    let mut vector: PostDataHolder = serde_json::from_str(&data).unwrap();

    vector.post_data.push(json);

    // Log the target triples recvd
    let fd = OpenOptions::new()
        .append(false)
        .write(true)
        .open("data.json")
        .unwrap();
    serde_json::to_writer_pretty(fd, &vector).unwrap();


    // so basically convert the thing to a target_triple
    // here and return it as a response.
    // need more data to know which target triple maps
    // to which architectures first.

    "wow"
}

pub async fn send_binary(
    Extension(repo_name): Extension<String>,
    Extension(cache): Extension<Arc<Mutex<Cache>>>,
    Extension(targets_compiling): Extension<TargetsCompiling>,
    Path(target_triple): Path<String>,
) -> Result<impl IntoResponse, String> {
    info!("Recieved a request to get target triple \"{target_triple}\"");

    if util::is_valid_target(&target_triple).await.is_none() {
        error!("Invalid target_triple: {target_triple} found!");
        return Err(format!("Invalid target triple: {target_triple}"));
    }


    // check if target is in cache
    // if true:
    //   return the path from cache.
    // else:
    //   check if target is currently being compiled
    //   if true:
    //     wait untill it is no longer being compiled
    //   else:
    //     add the target to the thing and proceed with the compilation


    // Ensure that target is not in cache already
    // if it is in cache, return the file early
    let cache_guard = cache.lock().await;
    if let Some(path) = cache_guard.get(&target_triple) {
        let path_but_string = &path.into_os_string().into_string().unwrap();
        debug!("Found path: {path_but_string} in cache");
        let name = path_but_string
            .split('/')
            .last()
            .unwrap()
            .to_string();
        info!("Returning file.");
        return util::return_file(&target_triple, &name).await;
    }
    drop(cache_guard);

    let mut being_compiled = targets_compiling.lock().await;
    let path_to_executable =  if being_compiled.contains(&target_triple) {
        // Compilation is currently occuring.

        // drop the guard so someone else (hopefully the one compiling)
        // can take it and finish the occuring compilation
        drop(being_compiled);

        // busy wait for the target to leave the vector (be finished compiling)
        debug!("Waiting on compilation lock.");
        loop {
            sleep(Duration::new(1,4)).await;
            let guard = targets_compiling.lock().await;
            if !guard.contains(&target_triple) {
                break;
            }
            drop(guard);
        }
        debug!("compilation finished, proceeding.");

        // Sleep a bit extra just to be extra sure its made it into cache!
        sleep(Duration::new(0, 2)).await;

        // get the (hopefully) compiled target from the cache
        cache.lock().await.get(&target_triple).unwrap()
    } else {
        // Compilation is not occuring, needs to be done.

        debug!("Pushing target triple to lock vector");
        // add the target_triple to the vector to indicate that it is being compiled.
        being_compiled.push(target_triple.clone());

        // Drop the mutex to allow others trying to compile the same target access.
        drop(being_compiled);

        // Clone the repo 
        info!("Cloning repo to: \"{repo_name}/{target_triple}\"");
        if let Err(e) = util::clone_repo(&repo_name, &target_triple).await {
            error!(e);
            return Err(e);
        }

        // Compile the target, return the entire path to the the executable
        info!("{target_triple} is not in cache, adding and compiling it now!");
        let executable_path = util::compile(&target_triple).await?;

        info!("Compiled, now Inserting {target_triple} into cache");
        // NOTE: this might still be premature since we have not called
        //       return_file() but i can solve that later in that case
        debug!("Waiting on cache lock");
        cache.lock().await.insert(
            target_triple.clone(),
            executable_path.clone(),
        );

        // Remove the compiled target_triple from the vector
        let mut being_compiled = targets_compiling.lock().await;
        being_compiled.retain(|s| s != &target_triple);
        // (drop the lock automatically)

        executable_path
    };

    let name = path_to_executable
        .into_os_string()
        .into_string()
        .unwrap()
        .split('/')
        .last()
        .unwrap()
        .to_string();

    info!("Returning file.");
    util::return_file(&target_triple, &name).await
}

pub async fn status(
) -> impl IntoResponse {
    "status here"
}
