use hashbrown::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;
use std::time::Instant;

/// The callback to run when a item goes out of the cache.
type Callback = fn(String);

/// A piece of data for usage in the [Cache](`Cache`).
struct Data {
    /// The path to the directory holding the code.
    path: PathBuf,

    /// The point at which the [Data] got created.
    creation: Instant,
}

type DB = Arc<Mutex<HashMap<String, Data>>>;

/// The main `Cache` structure.
#[derive(Clone)]
pub struct Cache {
    /// The [hashmap](`hashbrown::HashMap`) for storing the [Data](`Data`) in.
    hmap: DB,
}

impl Cache {
    /// Creates a new [Cache].
    ///
    /// * data_timeout: [Duration](`std::time::Duration`). How long each data should live inside the [Cache].
    /// * callback: A optional [Callback](`Callback`) for executing some code when a piece of [Data](`Data`) goes out of the [Cache].
    pub async fn new(data_timeout: Duration, callback: Option<Callback>) -> Self {
        let hmap: DB = Arc::new(Mutex::new(HashMap::new()));

        // For each new cache, spawn a loop which erases all data when it excedes the deadlines.
        // Only do this when the duration is greater than 0, 0 should mean no timeout.
        if !data_timeout.is_zero() {
            let h = hmap.clone();
            tokio::spawn(async move {
                // Vec for storing the dead (timed out) keys from the hashmap.
                // This looks really weird since we allocate the vec at new-time
                // when the hashmap does not have any elements in it...?
                let mut dead: Vec<String> = Vec::with_capacity(h.lock().unwrap().len());

                loop {
                    // Let the function loop on a interval.
                    tokio::time::sleep(Duration::new(0, 1)).await;

                    let mut map = h.lock().unwrap();

                    // Check which pieces of cache data overstayed their welcome (have a expired deadline).
                    // add them to the dead vector for easy removal, dont this way to appease the
                    // borrow checker...
                    for (k, data) in map.iter() {
                        if data.creation.elapsed() > data_timeout {
                            dead.push(k.to_string());
                        }
                    }

                    // Remove all the now dead (timed out) pieces of data from the hashmap.
                    for key in &dead {
                        if let Some(f) = callback {
                            f(key.clone());
                        }

                        map.remove_entry(key);
                    }

                    // Reset the array for storing new dead data.
                    dead.clear();
                }
            });
        }

        Cache { hmap }
    }

    /// Gets the item matching [k] from the [Cache](`Cache`).
    /// Updates the [Data](`Data`)'s creation time so that it does not timeout.
    pub fn get(&self, k: &String) -> Option<PathBuf> {
        {
            let mut hmap = self.hmap.lock().unwrap();
            let v = hmap.get_mut(k);
            if let Some(mut data) = v {
                data.creation = Instant::now();
            }
        }

        self.hmap.lock().unwrap().get(k).map(|v| (*v).path.clone())
    }

    /// Inserts a [k] and [v] into the [Cache](`Cache`).
    pub fn insert(&mut self, k: String, v: PathBuf) {
        let d = Data {
            path: v,
            creation: Instant::now(),
        };

        self.hmap.lock().unwrap().insert(k, d);
    }
}
