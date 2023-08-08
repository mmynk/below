use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

pub type NicMap = BTreeMap<String, NicStats>;
pub type CustomStats = HashMap<String, u64>;
pub type QueueStatsVec = Vec<QueueStats>;

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct EthtoolStats {
    pub nic: Option<NicMap>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct NicStats {
    pub queue: Option<QueueStatsVec>,
    pub tx_timeout: Option<u64>,
    pub custom_stats: Option<CustomStats>,
}

#[derive(Default, Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct QueueStats {
    pub rx_bytes: Option<u64>,
    pub tx_bytes: Option<u64>,
    pub rx_count: Option<u64>,
    pub tx_count: Option<u64>,
    pub tx_missed_tx: Option<u64>,
    pub tx_unmask_interrupt: Option<u64>,
    pub custom_stats: Option<CustomStats>,
}

pub fn insert_stat(stat: &mut QueueStats, name: &str, value: u64) {
    match name {
        "rx_bytes" => stat.rx_bytes = Some(value),
        "tx_bytes" => stat.tx_bytes = Some(value),
        "rx_cnt" => stat.rx_count = Some(value),
        "tx_cnt" => stat.tx_count = Some(value),
        "tx_missed_tx" => stat.tx_missed_tx = Some(value),
        "tx_unmask_interrupt" => stat.tx_unmask_interrupt = Some(value),
        _ => {
            stat.custom_stats.as_mut().unwrap().insert(name.to_string(), value);
        },
    };
}
