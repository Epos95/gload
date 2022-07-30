use axum::{
    body::{self, Full},
    extract::Path,
    response::{IntoResponse, Response},
    Extension, Json,
};
use http::StatusCode;
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tokio::{fs::File, io::AsyncReadExt, sync::Mutex};
use tracing::{error, info};

use crate::{cache::Cache, compilation_state, CompilationProgress};
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

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct PostData {
    os: String,
    os_version: String,
    user_agent: String,
}

// From the data sent here the server should respond with a token for which specific file to download
// the token can then be used by GETting a route with the token as a parameter or something :D
// This token could probably be the target_triple, which would prolly work out nicely.
pub async fn recv(Json(json): Json<PostData>) -> impl IntoResponse {
    // maybe write
    info!("Recvd: {json:#?}");

    "wow"
}

pub async fn send_binary(
    Extension(cache): Extension<Arc<Mutex<Cache>>>,
    Extension(currently_compiling): Extension<CurrentlyCompiling>,
    Extension(compilation_state): Extension<CompilationProgress>,
    Path(target_triple): Path<String>,
) -> Result<impl IntoResponse, String> {
    info!("Recieved a request to get target triple \"{target_triple}\"");

    if (util::is_valid_target(&target_triple).await).is_none() {
        error!("Invalid target_triple: {target_triple} found!");
        return Err(format!("Invalid target triple: {target_triple}"));
    }

    // If target is not in cache
    //   Check if target_triple in `currently_compiling` vector
    //     if yes, busy wait untill it is not
    //   else
    //     add target to vector
    //     compile stuff
    //     remove target from vector

    // Wait on mutex first
    let _guard = currently_compiling.lock().await;
    drop(_guard);

    // Check if in cache
    let mut res = cache.lock().await.get(&target_triple);
    if let None = res {
        // is not in cache, needs to compile
        let _guard = currently_compiling.lock().await;

        let s = util::compile(&target_triple, cache, compilation_state).await?;
        res = Some(PathBuf::from(s));
    }

    let name = res
        .unwrap()
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
    Extension(compilation_state): Extension<CompilationProgress>,
) -> impl IntoResponse {
    let guard = compilation_state.lock().await;

    format!("{};{}", guard.message, guard.progress)
}
