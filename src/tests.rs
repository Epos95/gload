#![cfg(test)]

use crate::cache;
use crate::cache::Cache;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

#[tokio::test]
async fn cache_insert() {
    let mut c = Cache::new(Duration::new(1, 0), None).await;
    c.insert("root".to_string(), PathBuf::from("/"));

    println!("Accessing!");
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));
}

#[tokio::test]
async fn cache_timeout1() {
    let mut c = Cache::new(Duration::new(0, 1), None).await;
    c.insert("root".to_string(), PathBuf::from("/"));

    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(1, 0)).await;

    let r = c.get(&"root".to_string());
    assert_eq!(r, None);
}

#[tokio::test]
async fn cache_timeout2() {
    let mut c = Cache::new(Duration::new(0, 3), None).await;
    c.insert("root".to_string(), PathBuf::from("/"));

    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));
    c.insert("home".to_string(), PathBuf::from("/home/epos/"));

    tokio::time::sleep(Duration::new(1, 0)).await;

    let r = c.get(&"root".to_string());
    assert_eq!(r, None);
    let r = c.get(&"home".to_string());
    assert_eq!(r, None);
}

#[tokio::test]
async fn cache_timeout3() {
    let mut c = Cache::new(Duration::new(0, 3), None).await;
    c.insert("root".to_string(), PathBuf::from("/"));

    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));
    c.insert("home".to_string(), PathBuf::from("/home/epos/"));

    tokio::time::sleep(Duration::new(1, 0)).await;

    let r = c.get(&"root".to_string());
    assert_eq!(r, None);
    let r = c.get(&"home".to_string());
    assert_eq!(r, None);

    c.insert("home".to_string(), PathBuf::from("/home/epos/"));
    let r = c.get(&"home".to_string());
    assert_eq!(r, Some(PathBuf::from("/home/epos/")));
}

#[tokio::test]
async fn cache_keepalive() {
    let mut c = Cache::new(Duration::new(3, 0), None).await;
    c.insert("root".to_string(), PathBuf::from("/"));

    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(1, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(1, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(1, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(1, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));
}

#[tokio::test]
async fn cache_keepalive2() {
    let mut c = Cache::new(Duration::new(5, 0), None).await;
    c.insert("root".to_string(), PathBuf::from("/"));

    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(3, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(3, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(3, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(3, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));
}

#[tokio::test]
async fn cache_callback() {
    let cb: cache::Callback = Box::new(|x| {
        let string = format!("{x} just went out cache!");
        let mut file = File::create("/tmp/testing_cache_thingy").unwrap();
        file.write_all(string.as_bytes()).unwrap();
    });

    let mut c = Cache::new(Duration::new(1, 0), Some(cb)).await;
    c.insert("root".to_string(), PathBuf::from("/"));

    tokio::time::sleep(Duration::new(1, 1)).await;

    let mut file = File::open("/tmp/testing_cache_thingy").unwrap();
    let mut buf = String::new();

    file.read_to_string(&mut buf).unwrap();
    assert!(buf.contains("root"));
}

#[tokio::test]
async fn cache_callback2() {
    File::create("/tmp/testing_cache_thingy2").unwrap();
    let cb: cache::Callback = Box::new(|x| {
        let string = format!("{x} just went out cache!");
        let mut file = File::create("/tmp/testing_cache_thingy2").unwrap();
        file.write_all(string.as_bytes()).unwrap();
    });

    let mut c = Cache::new(Duration::new(5, 0), Some(cb)).await;
    c.insert("root".to_string(), PathBuf::from("/"));

    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(3, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(3, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(3, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    tokio::time::sleep(Duration::new(3, 0)).await;
    let r = c.get(&"root".to_string());
    assert_eq!(r, Some(PathBuf::from("/")));

    let mut file = File::open("/tmp/testing_cache_thingy2").unwrap();
    let mut buf = String::new();

    file.read_to_string(&mut buf).unwrap();
    assert!(!buf.contains("root"));
}

#[tokio::test]
async fn cache_crash() {
    let mut c = Cache::new(Duration::new(3, 0), None).await;

    for i in 0..2000 {
        c.insert(format!("root{i}").to_string(), PathBuf::from("/"));
    }

    tokio::time::sleep(Duration::new(5, 0)).await;
}
