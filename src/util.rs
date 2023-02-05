use std::{fs, io::ErrorKind, path::PathBuf, process::Stdio};
use tokio::io::AsyncReadExt;

use axum::body::StreamBody;
use http::{header, HeaderMap, HeaderValue};
use tokio::{fs::File, process::Command};
use tokio_util::io::ReaderStream;
use tracing::{debug, error};

pub async fn is_valid_target(target_triple: &String) -> Option<String> {
    debug!("Trying to validate target: {target_triple}");
    // Check if toolchain is installed,
    // if installed, just return it
    // it not installed, try to install it
    // return toolchain string on success, error on failure
    let results = Command::new("rustup")
        .arg("toolchain")
        .arg("list")
        .output()
        .await
        .ok()?;

    let output = std::str::from_utf8(&results.stdout).ok()?;
    let toolchain_exists = output.contains(target_triple);

    if !toolchain_exists {
        debug!("Toolchain does not exist, adding now.");
        // add the toolchain
        // This only adds the toolchain, not installed...
        //
        // this is what has worked elsewhere (for windows machines)
        // rustup target add x86_64-pc-windows-gnu
        // rustup toolchain install stable-x86_64-pc-windows-gnu
        let results = Command::new("rustup")
            .arg("target")
            .arg("add")
            .arg(target_triple)
            .status()
            .await
            .ok()?;

        let status_code = results.code()?;
        if status_code > 0 {
            return None;
        }
    }

    Some(target_triple.to_owned())
}

/// Gets file contents and returns them as a axum-returnable type.
///
/// Uses the given `target_triple` and `executable_name` to find the executable file to return and
/// creates a axum-returnable representing the executable file.
pub async fn return_file(
    target_triple: &String,
    executable_name: &String,
    compilation_directory: &PathBuf,
) -> Result<(HeaderMap, StreamBody<ReaderStream<File>>), String> {
    // At this point we can be sure that the file exists
    // and that we can grab it safely (hopefully)!
    let fname = compilation_directory
        .join(target_triple)
        .join("target")
        .join(target_triple)
        .join("release")
        .join(executable_name);

    debug!("Returning filename: {fname:?}");
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

    Ok((headers, body))
}

/// Destroys and restores the compilation_directory folder so that it can be used again.
pub fn restore_compilation_directory(compilation_directory: &PathBuf) -> Result<(), String> {
    debug!("Checking repo availiability...");

    if let Err(e) = fs::remove_dir_all(compilation_directory) {
        let kind = e.kind();

        // Handle errors which are recoverable (such as NotFound)
        // discreetly, otherwise panic. This is because NotFound
        // really doesnt matter to us at this point
        match kind {
            ErrorKind::NotFound => {}
            _ => {
                return Err(e.to_string());
            }
        }
    }

    if let Err(e) = fs::create_dir(compilation_directory) {
        return Err(e.to_string());
    }

    debug!("Succesfully restored repo");

    Ok(())
}

/// Should clone the `origin_url` into `compilation_directory/target_name`.
pub async fn clone_repo(
    origin_url: &String,
    target_name: &String,
    compilation_directory: &PathBuf,
) -> Result<(), String> {
    let git_output = Command::new("git")
        .arg("clone")
        .arg(origin_url)
        .arg(compilation_directory.join(target_name))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();

    // NOTE: could maybe simplify this match by using .and_then() or something
    //       since we just need to apply a function to the value if its Ok(),
    //       otherwise we should print the error.
    match git_output {
        Ok(mut o) => {
            // Check the git output
            if o.wait().await.unwrap().code().unwrap() != 0 {
                error!("Error when trying to clone git repository \"{origin_url}\"");
                return Err(
                    "Error when trying to clone git repository, please try again later!"
                        .to_string(),
                );
            }
        }
        Err(e) => {
            error!("Error cloning the repo:  {e:?}");
            return Err(format!("Error cloning the repo: {e:?}"));
        }
    }

    Ok(())
}

/// Tries to compile for the specified target_triple.
/// Returns the path to the compiled executable file.
pub async fn compile(
    target_triple: &String,
    compilation_directory: &PathBuf,
    config: &Config
) -> Result<PathBuf, String> {
    let (stdout, stderr) = if config.debug {
        (Stdio::inherit(), Stdio::inherit())
    } else {
        (Stdio::null(), Stdio::null())
    };

    // need some way to get the "building" part of cargo output
    let s = Command::new("cross")
        .arg("b")
        .arg("--release")
        .arg("--manifest-path")
        .arg(compilation_directory.join(target_triple).join("Cargo.toml"))
        .arg(format!("--target={target_triple}"))
        .stdout(stdout)
        .stderr(stderr)
        .status()
        .await
        .expect("Failed to use cross.");

    // NOTE: This does NOT seem that healthy tbh
    if let Some(code) = s.code() {
        debug!("return code: {code}");
        if code > 0 {
            error!("Cross returned error code: {code}");
            return Err(format!("Cross return error code: {code}"));
        }
    }

    let executable_name = if let Some(e) = config.binary_name.clone() {
        e.clone()
    } else {
        get_executable_name(target_triple, compilation_directory).await
    };

    let executable_path = compilation_directory
        .join("target")
        .join(target_triple)
        .join("release")
        .join(executable_name);

    Ok(executable_path)
}

/// Get a executables name via Cargo.toml to be /absolutely/ sure its the corrent name.
pub async fn get_executable_name(target_triple: &String, compilation_directory: &PathBuf) -> String {
    let mut file_descriptor = File::open(compilation_directory.join(target_triple).join("Cargo.toml"))
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
        debug!("detected windows, adding .exe suffix");
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

#[derive(Clone, Debug)]
pub struct Config {
    debug: bool,
    binary_name: Option<String>,
}

impl Config {
    pub fn new(debug: bool, binary_name: Option<String>) -> Self {
        Config {
            debug,
            binary_name,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self { debug: false, binary_name: None }
    }
}
