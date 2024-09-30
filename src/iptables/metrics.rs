use std::collections::HashMap;

use anyhow::Result;
use prometheus::{IntCounterVec, IntGaugeVec, Opts, Registry};
use tracing::{debug, trace};

use crate::{cli::ScrapeTarget, iptables::IptablesState};

pub(crate) struct TargetMetricsIptables {
    chains_total: IntGaugeVec,
    rules_total: IntGaugeVec,
    chain_bytes_total: IntCounterVec,
    chain_packets_total: IntCounterVec,
    comment_bytes_total: IntCounterVec,
    comment_packets_total: IntCounterVec,
    rule_bytes_total: IntCounterVec,
    rule_packets_total: IntCounterVec,
}

impl TargetMetricsIptables {
    fn update(&mut self, state: &IptablesState) {
        for t in &state.tables {
            let ct = self.chains_total.with_label_values(&[&t.name]);
            ct.set(t.chains.len() as i64);

            for (_, c) in t.chains.iter() {
                let cbt = self
                    .chain_bytes_total
                    .with_label_values(&[&t.name, &c.name, c.policy()]);
                let diff = c.counter.bytes() - cbt.get();
                cbt.inc_by(diff);

                let cpt =
                    self.chain_packets_total
                        .with_label_values(&[&t.name, &c.name, c.policy()]);
                let diff = c.counter.packets() - cpt.get();
                cpt.inc_by(diff);

                let rt = self.rules_total.with_label_values(&[&t.name, &c.name]);
                rt.set(c.rules.len() as i64);

                for r in &c.rules {
                    let rpt = self
                        .rule_packets_total
                        .with_label_values(&[&t.name, &c.name, &r.rule]);
                    let diff = r.counter.packets() - rpt.get();
                    rpt.inc_by(diff);

                    let rbt = self
                        .rule_bytes_total
                        .with_label_values(&[&t.name, &c.name, &r.rule]);
                    let diff = r.counter.bytes() - rbt.get();
                    rbt.inc_by(diff);

                    if let Some(cmt) = &r.comment {
                        let cpt = self.comment_packets_total.with_label_values(&[
                            &t.name,
                            &c.name,
                            &cmt.comment,
                        ]);
                        let diff = cmt.counter.packets() - cpt.get();
                        cpt.inc_by(diff);

                        let cbt = self.comment_bytes_total.with_label_values(&[
                            &t.name,
                            &c.name,
                            &cmt.comment,
                        ]);
                        let diff = cmt.counter.packets() - cbt.get();
                        cbt.inc_by(diff);
                    }
                }
            }
        }
    }
}

pub(crate) struct MetricsIptables {
    map: HashMap<String, TargetMetricsIptables>,
}

impl MetricsIptables {
    pub(crate) fn new(targets: &[ScrapeTarget], r: &Registry) -> Result<Self> {
        trace!("MetricsIptables::new");

        let mut map = HashMap::new();
        for tgt in targets {
            let prefix = tgt.as_ref().replace('-', "_");

            let chains_total = IntGaugeVec::new(
                Opts::new(
                    &format!("{prefix}_chains_total"),
                    "Total number of chains in a table",
                ),
                &["table"],
            )?;

            let rules_total = IntGaugeVec::new(
                Opts::new(
                    &format!("{prefix}_rules_total"),
                    "Total number of rules in a chain in a table",
                ),
                &["table", "chain"],
            )?;

            let chain_bytes_total = IntCounterVec::new(
                Opts::new(
                    &format!("{prefix}_chain_bytes_total"),
                    "Total bytes flowing through a given chain",
                ),
                &["table", "chain", "policy"],
            )?;

            let chain_packets_total = IntCounterVec::new(
                Opts::new(
                    &format!("{prefix}_chain_packets_total"),
                    "Total packets flowing through a given chain",
                ),
                &["table", "chain", "policy"],
            )?;

            let rule_bytes_total = IntCounterVec::new(
                Opts::new(
                    &format!("{prefix}_rule_bytes_total"),
                    "Total bytes matching a given rule",
                ),
                &["table", "chain", "rule"],
            )?;

            let rule_packets_total = IntCounterVec::new(
                Opts::new(
                    &format!("{prefix}_rule_packets_total"),
                    "Total packets matching a given rule",
                ),
                &["table", "chain", "rule"],
            )?;

            let comment_bytes_total = IntCounterVec::new(
                Opts::new(
                    &format!("{prefix}_comment_bytes_total"),
                    "Total bytes matching a given comment inside a rule",
                ),
                &["table", "chain", "comment"],
            )?;

            let comment_packets_total = IntCounterVec::new(
                Opts::new(
                    &format!("{prefix}_comment_packets_total"),
                    "Total packets matching a given comment inside a rule",
                ),
                &["table", "chain", "comment"],
            )?;

            debug!("Registering {prefix} metrics");
            r.register(Box::new(chain_bytes_total.clone()))?;
            r.register(Box::new(chain_packets_total.clone()))?;
            r.register(Box::new(comment_bytes_total.clone()))?;
            r.register(Box::new(comment_packets_total.clone()))?;
            r.register(Box::new(rule_bytes_total.clone()))?;
            r.register(Box::new(rule_packets_total.clone()))?;
            r.register(Box::new(rules_total.clone()))?;
            r.register(Box::new(chains_total.clone()))?;
            map.insert(
                tgt.to_string(),
                TargetMetricsIptables {
                    chains_total,
                    rules_total,
                    chain_bytes_total,
                    chain_packets_total,
                    comment_bytes_total,
                    comment_packets_total,
                    rule_bytes_total,
                    rule_packets_total,
                },
            );
        }

        Ok(Self { map })
    }

    pub(crate) fn update(&mut self, tgt: ScrapeTarget, state: &IptablesState) {
        if let Some(tgt_metrics) = self.map.get_mut(tgt.as_ref()) {
            tgt_metrics.update(state);
        }
    }
}
