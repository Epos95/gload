use axum::{
    body::{self, Full},
    extract::Path,
    response::{IntoResponse, Response},
    Extension, Json,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, fs::{OpenOptions, self}};
use tokio::{fs::File, io::AsyncReadExt, sync::Mutex};
use tracing::{error, info};

use crate::cache::Cache ;
use crate::util;
use crate::CurrentlyCompiling;

pub async fn get_target() -> impl IntoResponse {
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
    Extension(currently_compiling): Extension<CurrentlyCompiling>,
    Path(target_triple): Path<String>,
) -> Result<impl IntoResponse, String> {
    info!("Recieved a request to get target triple \"{target_triple}\"");

    if util::is_valid_target(&target_triple) {
        error!("Invalid target_triple: {target_triple} found!");
        return Err(format!("Invalid target triple: {target_triple}"));
    }

    // Wait on mutex first
    let _guard = currently_compiling.lock().await;
    drop(_guard);

    let guard = cache.lock().await;
    let potential_path = guard.get(&target_triple);
    drop(guard);

    // Check if in cache
    let result_path = match potential_path {
        Some(r) => r,
        None => {
            // is not in cache, needs to compile
            let _guard = currently_compiling.lock().await;

            let executable_path = util::compile(&target_triple).await?;

            info!("Compiled, now Inserting {target_triple} into cache");

            // Update the cache accordingly
            cache.lock().await.insert(
                target_triple.clone(),
                executable_path.clone(),
            );

            executable_path
        }
    };

    let name = result_path
        .into_os_string()
        .into_string()
        .unwrap()
        .split("/")
        .last()
        .unwrap()
        .to_string();

    util::return_file(&target_triple, &name).await
}

pub async fn status(
) -> impl IntoResponse {
    "status here"
}
