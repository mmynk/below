mod errors;
mod ethtool;
mod types;

use std::collections::{BTreeMap, HashMap};

use errors::Error;
pub use types::*;

pub type Result<T> = std::result::Result<T, errors::Error>;

fn is_queue_stat(name: &str) -> bool {
    name.starts_with("queue_")
}

fn parse_queue_stat(name: &str) -> Result<(usize, &str)> {
    let stat_segments: Vec<&str> = name.splitn(3, '_').collect();
    match stat_segments[1].parse::<usize>() {
        Ok(queue_id) => Ok((queue_id, stat_segments[2])),
        Err(_) => Err(Error::ParseError(String::from("Failed to parse queue id"))),
    }
}

fn translate_stats(stats: Vec<(String, u64)>) -> Result<NicStats> {
    let mut nic_stats = NicStats::default();
    let mut custom_props = HashMap::new();
    let mut queue_stats_map = BTreeMap::new();  // we want to preserve the order of the queues
    for (name, value) in stats {
        if is_queue_stat(&name) {
            match parse_queue_stat(&name) {
                Ok((queue_id, stat)) => {
                    if !queue_stats_map.contains_key(&queue_id) {
                        let queue_stat = QueueStats {
                            ..Default::default()
                        };
                        queue_stats_map.insert(queue_id, queue_stat);
                    }
                    let qstat = queue_stats_map.get_mut(&queue_id).unwrap();
                    insert_stat(qstat, stat, value);
                },
                Err(err) => return Err(err),
            }
        } else {
            match name.as_str() {
                "tx_timeout" => nic_stats.tx_timeout = Some(value),
                other => {
                    custom_props.insert(other.to_string(), value);
                },
            }
        }
    }

    let mut queue_stats = None;
    if !queue_stats_map.is_empty() {
        let mut qstats = Vec::with_capacity(queue_stats_map.len());
        for (_, stats) in queue_stats_map {
            qstats.push(stats);
        }
        queue_stats = Some(qstats);
    }

    nic_stats.queue = queue_stats;

    Ok(nic_stats)
}

pub struct EthtoolReader;

impl EthtoolReader {
    // TODO: `new` should take Ethtool as a parameter so that we can mock it in tests
    pub fn new() -> Self {
        Self {}
    }

    /// Read stats for a single NIC driver identified by `if_name`
    fn read_nic_stats(&self, if_name: &str) -> Result<NicStats> {
        match ethtool::Ethtool::init(if_name).stats() {
            Ok(stats) => {
                translate_stats(stats)
            }
            Err(error) => Err(error),
        }
    }

    pub fn read_stats(&self) -> Result<NetStats> {
        let mut nic_stats_map = NicMap::new();
        // TODO: fetch the list of interfaces from the kernel
        let ifs = vec![
            String::from("ens5")
            ];
        for if_name in ifs {
            if let Ok(nic_stats) = self.read_nic_stats(&if_name) {
                nic_stats_map.insert(if_name.to_string(), nic_stats);
            }
        }

        Ok(NetStats { nic: Some(nic_stats_map) })
    }
}

#[cfg(test)]
mod tests {
    use crate::EthtoolReader;

    #[test]
    fn test_read_tc_stats() {
        let reader = EthtoolReader::new();
        let result = reader.read_stats();
        assert!(result.is_ok());

        let net_stats = result.unwrap();
        assert!(net_stats.nic.is_some());
        assert!(net_stats.nic.unwrap()["ens5"].tx_timeout.is_some());
    }
}
