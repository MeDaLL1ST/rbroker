use lazy_static::lazy_static;
use prometrics_sb::axpromlib::{Selfcounter, Selfgauge};
use prometrics_sb::{create_counter, create_gauge};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{
    mpsc::{self, Receiver, Sender},
    RwLock,
};

lazy_static! {
    pub static ref ALL_USES: Selfcounter =
        create_counter!("all_uses", "The total number of active uses");
    pub static ref INFO_USES: Selfcounter =
        create_counter!("info_uses", "The total number of info uses");
    pub static ref ALL_CONS: Selfgauge =
        create_gauge!("conns", "All current websocket connections");
    pub static ref STORE: Store = Store::default();
}

///The basic data structure of the broker
pub struct Store {
    uses: Arc<RwLock<HashMap<String, usize>>>,
    updates: Arc<RwLock<HashMap<String, VecDeque<(Sender<String>, usize)>>>>,
}

impl Store {
    pub fn new() -> Self {
        Store {
            uses: Arc::new(RwLock::new(HashMap::new())),
            updates: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    ///Sending a value to a channel
    pub fn rpush(&'static self, key: String, value: String) {
        tokio::task::spawn(async move {
            let mut updates_guard = self.updates.write().await;

            if let Some(queue) = updates_guard.get_mut(&key) {
       
                if let Some((sender, id)) = queue.pop_front() {
                    if sender.send(value.clone()).await.is_ok() {
                        queue.push_back((sender, id));
                    }
                }
            }
        });
    }
    ///Key subscription
    pub async fn get_updates(&self, key: String) -> (Receiver<String>, usize) {

        let mut uses_guard = self.uses.write().await;
        let mut updates_guard = self.updates.write().await;

        let use_count = uses_guard.entry(key.clone()).or_insert(0);
        *use_count += 1;

        let (tx, rx): (Sender<String>, Receiver<String>) = mpsc::channel(1);

        let unique_id = *use_count;

        if let Some(queue) = updates_guard.get_mut(&key) {
            queue.push_back((tx, unique_id));
        } else {
            let mut queue = VecDeque::new();
            queue.push_back((tx, unique_id));
            updates_guard.insert(key.clone(), queue);
        }

        (rx, unique_id)
    }
    ///Decrementing key usage and clearing memory after all unsubscriptions
    pub async fn dec_key(&self, key: String, id: usize) {
        let mut uses_guard = self.uses.write().await;
        let mut updates_guard = self.updates.write().await;
        if let Some(use_count) = uses_guard.get_mut(&key) {
            if *use_count > 0 {
                *use_count -= 1;

                if let Some(queue) = updates_guard.get_mut(&key) {
                    if let Some(pos) = queue.iter().position(|(_, sender_id)| *sender_id == id) {
                        queue.remove(pos);
                    }
                }
            }

            if *use_count == 0 {
                uses_guard.remove(&key);
                updates_guard.remove(&key);
            }
        }
    }
    ///Getting the number of subscribers per key
    pub async fn info(&self, key: String) -> usize {
        let uses_guard = self.uses.read().await;
        let _ = self.updates.read().await;
        if let Some(keyi) = uses_guard.get(&key) {
            *keyi
        } else {
            0
        }
    }
    ///Getting all keys with existing subscribers
    pub async fn list(&self) -> Vec<String> {
        let updates_guard = self.updates.read().await;
        let _ = self.uses.read().await;
        let keys: Vec<String> = updates_guard.keys().cloned().collect();
        keys
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new()
    }
}
