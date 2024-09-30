use std::{collections::HashMap, num::Saturating};

use anyhow::Result;
use prometheus::{IntCounterVec, IntGaugeVec, Opts, Registry};
use tracing::{debug, trace};

use crate::{cli::ScrapeTarget, ipset::IpsetState};

pub(crate) struct TargetMetricsIpset {
    entries_total: IntGaugeVec,
    ips_total: IntGaugeVec,
}

impl TargetMetricsIpset {
    fn update(&mut self, state: &IpsetState) {
        for l in &state.lists {
            let c = self.entries_total.with_label_values(&[&l.name]);
            c.set(l.entries.len() as i64);
        }
    }
}

pub(crate) struct MetricsIpset {
    map: HashMap<String, TargetMetricsIpset>,
}

impl MetricsIpset {
    pub(crate) fn new(targets: &[ScrapeTarget], r: &Registry) -> Result<Self> {
        trace!("MetricsIpset::new");

        let mut map = HashMap::new();
        for tgt in targets {
            if !matches!(tgt, ScrapeTarget::Ipset) {
                continue;
            }
            let prefix = String::from("ipset");
            let entries_total = IntGaugeVec::new(
                Opts::new(
                    &format!("{prefix}_entries_total"),
                    "Total number of entries in the ipset",
                ),
                &["list"],
            )?;

            let ips_total = IntGaugeVec::new(
                Opts::new(
                    &format!("{prefix}_ips_total"),
                    "Total number individual IPs",
                ),
                &["list"],
            )?;

            r.register(Box::new(entries_total.clone()))?;
            r.register(Box::new(ips_total.clone()))?;

            map.insert(
                prefix,
                TargetMetricsIpset {
                    entries_total,
                    ips_total,
                },
            );
        }

        Ok(Self { map })
    }

    pub(crate) fn update(&mut self, tgt: ScrapeTarget, state: &IpsetState) {
        if let Some(tgt_metrics) = self.map.get_mut(tgt.as_ref()) {
            tgt_metrics.update(state);
        }
    }
}
