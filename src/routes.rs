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
use tracing::{error, info};

use crate::cache::Cache ;
use crate::util;
use crate::TargetsCompiling;

pub async fn get_target() -> impl IntoResponse {
    // TODO: Need to always use the linux linker for windows
    //       e.h -gnu and not -msvc
    // Send the html of the page which gets the target triple

    let mut file = File::open("templates/index.html").await.unwrap();
    let mut html = String::new();
    file.read_to_string(&mut html).await.unwrap();

    Response::builder()
        .status(StatusCode::OK)
        .body(body::boxed(Full::from(html)))
        .unwrap()
}

#[derive(Debug, Deserialize, Serialize)]
#[allow(dead_code)]
pub struct PostData {
    os: String,
    os_version: String,
    user_agent: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct PostDataHolder{ post_data: Vec<PostData>}


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
    Extension(cache): Extension<Arc<Mutex<Cache>>>,
    Extension(targets_compiling): Extension<TargetsCompiling>,
    Path(target_triple): Path<String>,
) -> Result<impl IntoResponse, String> {
    info!("Recieved a request to get target triple \"{target_triple}\"");

    if util::is_valid_target(&target_triple).await.is_none() {
        error!("Invalid target_triple: {target_triple} found!");
        return Err(format!("Invalid target triple: {target_triple}"));
    }


    // Ensure that target is not in cache already
    // if it is in cache, return the file early
    let cache_guard = cache.lock().await;
    if let Some(path) = cache_guard.get(&target_triple) {
        let path_but_string = &path.into_os_string().into_string().unwrap();
        return util::return_file(&target_triple, path_but_string).await;
    }
    drop(cache_guard);


    // ENSURE THAT TARGET IS NOT IN CACHE
    // check if target is currently being compiled
    // if true:
    //   wait untill it is no longer being compiled
    // else:
    //   add the target to the thing and proceed with the compilation

    let mut being_compiled = targets_compiling.lock().await;
    let path_to_executable =  if being_compiled.contains(&target_triple) {
        // drop the guard so someone else (hopefully the one compiling)
        // can take it and finish the compilation
        drop(being_compiled);

        // busy wait for the target to leave the vector (be finished compiling)
        loop {
            sleep(Duration::new(0,4)).await;
            let guard = targets_compiling.lock().await;
            if !guard.contains(&target_triple) {
                break;
            }
            drop(guard);
        }

        // Sleep a bit extra just to be extra sure its made it into cache!
        sleep(Duration::new(0,2)).await;

        // get the (hopefully) compiled target from the cache
        cache.lock().await.get(&target_triple).unwrap()
    } else {
        // add the target_triple to the vector to indicate that it is being compiled.
        being_compiled.push(target_triple.clone());

        // Drop the mutex to allow others trying to compile the same target access.
        drop(being_compiled);

        // Compile the target, return the entire path to the the executable
        let executable_path = util::compile(&target_triple).await?;

        info!("Compiled, now Inserting {target_triple} into cache");
        // NOTE: this might still be premature since we have not called
        //       return_file() but i can solve that later in that case
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
        .split("/")
        .last()
        .unwrap()
        .to_string();

    let file = util::return_file(&target_triple, &name).await?;
    Ok(file)
}

pub async fn status(
) -> impl IntoResponse {
    "status here"
}
