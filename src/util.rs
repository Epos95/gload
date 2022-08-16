use std::{path::PathBuf, process::Stdio, fs, io::ErrorKind};
use tokio::io::AsyncReadExt;

use axum::body::StreamBody;
use http::{header, HeaderMap, HeaderValue};
use tokio::{fs::File, process::Command};
use tokio_util::io::ReaderStream;
use tracing::{error, info};

pub const REPO_LOCATION: &str = "repo_to_compile";

/// Validates a target triple by checking if the given target is a tier 1
/// supported rust target. (see https://doc.rust-lang.org/nightly/rustc/platform-support.html) for more information.
pub fn is_valid_target(target_triple: &String) -> bool {
    // Check if toolchain is installed,
    // if installed, just return it
    // it not installed, try to install it
    // return toolchain string on success, error on failure

    let valid_targets = vec![
        "aarch64-unknown-linux-gnu",
        "i686-pc-windows-gnu",
        "i686-pc-windows-msvc",
        "i686-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "x86_64-pc-windows-gnu",
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-gnu",
    ];

    valid_targets.contains(&target_triple.as_str())
}

/// Gets file contents and returns them as a axum-returnable type.
///
/// Uses the given `target_triple` and `executable_name` to find the executable file to return and
/// creates a axum-returnable representing the executable file.
pub async fn return_file(
    target_triple: &String,
    executable_name: &String,
) -> Result<(HeaderMap, StreamBody<ReaderStream<File>>), String> {
    // At this point we can be sure that the file exists
    // and that we can grab it safely (hopefully)!
    let fname = format!("{REPO_LOCATION}/target/{target_triple}/release/{executable_name}",);
    let file = match File::open(&fname).await {
        Ok(f) => f,
        Err(_) => {
            error!("Failed to open {fname:?}");
            return Err(format!("Error: Failed to open file: {fname:?}!"));
        }
    };

    // make the file into a body through axum and magic
    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);

    // Create appropriate headers
    let disposition =
        HeaderValue::from_str(&format!("attachment; filename={}", &executable_name)).unwrap();
    let ctype = HeaderValue::from_str("").unwrap();
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, ctype);
    headers.insert(header::CONTENT_DISPOSITION, disposition);

    info!("Returning exeutable: {executable_name}");
    Ok((headers, body))
}


pub async fn ensure_repo_exists(repo_name: &String, should_recompile: bool) -> Result<(), String> {
    info!("Checking repo availiability...");

    if !should_recompile {
        if PathBuf::from(REPO_LOCATION).exists() {
            let mut file_descriptor = File::open(format!("{REPO_LOCATION}/Cargo.toml"))
                .await
                .unwrap();

            let mut string = String::new();
            file_descriptor.read_to_string(&mut string).await.unwrap();
            let executable_name = string.split('\n').find(|s| s.contains("name")).unwrap();
            let executable_name = executable_name
                .split('=')
                .last()
                .unwrap()
                .replace('\"', "")
                .replace(' ', "");

            if repo_name.contains(&executable_name) {
                info!("Repo exists!");
                return Ok(());
            }
        }
    }

    if let Err(e) = fs::remove_dir_all(REPO_LOCATION) {
        let kind = e.kind();

        // Handle errors which are recoverable (such as NotFound)
        // discreetly, otherwise panic. This is because NotFound
        // really doesnt matter to us at this point
        match kind {
            ErrorKind::NotFound => {},
            _ => {Err(e).unwrap()}
        }
    }

    info!("Cloning repo to: \"{REPO_LOCATION}\"");
    let git_output = Command::new("git")
        .arg("clone")
        .arg(repo_name)
        .arg(REPO_LOCATION)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    // NOTE: could maybe simplifyy this match by using .and_then() or something
    //       since we just need to apply a function to the value if its Ok(),
    //       otherwise we should print the error.
    match git_output {
        Ok(mut o) => {
            // Check the git output
            if o.wait().await.unwrap().code().unwrap() != 0 {
                error!("Error when trying to clone git repository \"{repo_name}\"");
                return Err(
                    "Error when trying to clone git repository, please try again later!"
                        .to_string(),
                );
            }
        }
        Err(e) => {
            error!("Error: {e:?}");
            return Err(format!("Error: {e:?}"));
        }
    }

    Ok(())
}

/// Tries to compile for the specified target_triple.
/// Returns the path to the compiled executable file.
pub async fn compile(
    target_triple: &String,
) -> Result<PathBuf, String> {
    info!("{target_triple} is not in cache, adding and compiling it now!");

    // need some way to get the "building" part of cargo output
    let s = Command::new("cross")
        .arg("b")
        .arg("--release")
        .arg("--manifest-path")
        .arg(format!("{REPO_LOCATION}/Cargo.toml"))
        .arg(format!("--target={target_triple}"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .expect("Failed to use cross.");

    // NOTE: This does NOT seem that healthy tbh
    if let Some(code) = s.code() {
        if code > 0 {
            error!("Cross returned error code: {code}");
            return Err("Cross return error code: {code}".to_string());
        }
    }

    let executable_name = get_executable_name(target_triple).await;
    let executable_path = PathBuf::from(format!(
        "{REPO_LOCATION}/target/{target_triple}/release/{executable_name}"
    ));


    Ok(executable_path)
}

/// Get a executables name via Cargo.toml to be /absolutely/ sure its the corrent name.
pub async fn get_executable_name(target_triple: &String) -> String {
    let mut file_descriptor = File::open(format!("{REPO_LOCATION}/Cargo.toml"))
        .await
        .unwrap();
    let mut string = String::new();
    file_descriptor.read_to_string(&mut string).await.unwrap();
    let executable_name = string.split('\n').find(|s| s.contains("name")).unwrap();
    let mut executable_name = executable_name
        .split('=')
        .last()
        .unwrap()
        .replace('\"', "")
        .replace(' ', "");

    // Account for .exe extension on windows
    if target_triple.contains("windows") {
        executable_name.push_str(".exe");
    }

    executable_name
}

pub fn cross_not_found() -> bool {
    Command::new("cross")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_err()
}
