use super::*;

use ethtool::{EthtoolStats, NicStats, QueueStats};

#[derive(Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct EthtoolModel {
    #[queriable(subquery)]
    // TODO: not sure how to dump (render_config) correctly
    // for a map of vector, so testing with a single queue for now
    pub nic: BTreeMap<String, NicModel>,
}

impl EthtoolModel {
    pub fn new(sample: &EthtoolStats, last: Option<(&EthtoolStats, Duration)>) -> Self {
        let mut nic = BTreeMap::new();
        if let Some((l, d)) = last {
            if sample.nic == None || l.nic == None {
                return Self {nic};
            }

            let empty_map = BTreeMap::new();
            let s_nic_map = sample.nic.as_ref().unwrap_or(&empty_map);
            let l_nic_map = l.nic.as_ref().unwrap_or(&empty_map);

            for (interface, s_nic_stats) in s_nic_map {
                if let Some(l_nic_stats) = l_nic_map.get(interface) {
                    let _s_queue_stats = &s_nic_stats.queue;
                    let _l_queue_stats = &l_nic_stats.queue;

                    let mut nic_model = NicModel {
                        nic: SingleNicModel::new(
                            interface,
                            s_nic_stats,
                            Some((l_nic_stats, d))
                        ),
                        queues: Vec::new(),
                    };

                    if _s_queue_stats.is_some() && _l_queue_stats.is_some() {
                        let s_queue_stats_vec = _s_queue_stats.as_ref().unwrap();
                        let l_queue_stats_vec = _l_queue_stats.as_ref().unwrap();

                        // Vec<QueueStats> are always sorted on the queue id, so they can be zipped together
                        for (queue_id, (s_queue_stats, l_queue_stats)) in std::iter::zip(s_queue_stats_vec, l_queue_stats_vec).enumerate() {
                            let idx = queue_id as u32;
                            let queue_model = SingleQueueModel::new(
                                interface,
                                idx,
                                s_queue_stats,
                                Some((l_queue_stats, d))
                            );
                            nic_model.queues.push(queue_model);
                        }
                    }

                    // TODO: add custom stats
                    nic.insert(interface.to_string(), nic_model);
                }
            }
        }

        Self {nic}
    }
}

impl Nameable for EthtoolModel {
    fn name() -> &'static str {
        "ethtool"
    }
}

#[derive(Clone, Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct NicModel {
    #[queriable(subquery)]
    pub nic: SingleNicModel,
    #[queriable(subquery)]
    pub queues: Vec<SingleQueueModel>,
}

#[derive(Clone, Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct SingleNicModel {
    pub interface: String,
    pub tx_timeout_per_sec: Option<u64>,
    // TODO: add custom stats
    // pub custom_stats: BTreeMap<String, u64>,
}

impl Nameable for SingleNicModel {
    fn name() -> &'static str {
        "ethtool_nic"
    }
}

impl SingleNicModel {
    fn new(
        interface: &str,
        sample: &NicStats,
        last: Option<(&NicStats, Duration)>
    ) -> Self {
        SingleNicModel {
            interface: interface.to_string(),
            tx_timeout_per_sec: get_option_rate!(tx_timeout, sample, last),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize, below_derive::Queriable)]
pub struct SingleQueueModel {
    pub interface: String,
    pub queue_id: u32,
    pub rx_bytes_per_sec: Option<u64>,
    pub tx_bytes_per_sec: Option<u64>,
    pub rx_count_per_sec: Option<u64>,
    pub tx_count_per_sec: Option<u64>,
    pub tx_missed_tx: Option<u64>,
    pub tx_unmask_interrupt: Option<u64>,
    // TODO: add custom stats
    // pub custom_stats: Option<CustomStats>,
}

impl SingleQueueModel {
    fn new(
        interface: &str,
        queue_id: u32,
        sample: &QueueStats,
        last: Option<(&QueueStats, Duration)>
    ) -> Self {
        SingleQueueModel {
            interface: interface.to_string(),
            queue_id,
            rx_bytes_per_sec: get_option_rate!(rx_bytes, sample, last),
            tx_bytes_per_sec: get_option_rate!(tx_bytes, sample, last),
            rx_count_per_sec: get_option_rate!(rx_count, sample, last),
            tx_count_per_sec: get_option_rate!(tx_count, sample, last),
            tx_missed_tx: sample.tx_missed_tx,
            tx_unmask_interrupt: sample.tx_unmask_interrupt,
        }
    }
}

impl Nameable for SingleQueueModel {
    fn name() -> &'static str {
        "ethtool_queue"
    }
}
