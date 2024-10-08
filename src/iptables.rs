mod chain;
mod counter;
mod metrics;
mod rule;
mod table;

pub(crate) use chain::Chain;
pub(crate) use counter::Counter;
pub(crate) use metrics::MetricsIptables;
pub(crate) use rule::Rule;
pub(crate) use table::Table;

use std::{result::Result as StdResult, str::FromStr};

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::{cli::ScrapeTarget, error::IptablesError};

pub(crate) async fn iptables_save(tgt: ScrapeTarget) -> Result<String> {
    let cmd = format!("{tgt}-save");

    String::from_utf8(
        Command::new(&cmd)
            .arg("-c")
            .output()
            .await
            .with_context(|| format!("Failed to run {cmd}"))?
            .stdout,
    )
    .with_context(|| format!("Failed {cmd} output to valid UTF-8"))
}

#[allow(dead_code)]
#[derive(Copy, Clone, PartialEq, Debug)]
enum Policy {
    Accept,
    Drop,
}

impl FromStr for Policy {
    type Err = IptablesError;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match s {
            "Accept" | "ACCEPT" | "accept" => Ok(Self::Accept),
            "Drop" | "DROP" | "drop" => Ok(Self::Accept),
            _ => Err(IptablesError::InvalidPolicy(s.into())),
        }
    }
}

#[derive(Debug)]
pub(crate) struct IptablesState {
    tables: Vec<Table>,
}

impl IptablesState {
    pub(crate) fn new() -> Self {
        Self { tables: Vec::new() }
    }

    pub(crate) async fn parse<S: AsRef<str>>(&mut self, out: S) -> Result<()> {
        let mut table: Option<Table> = None;
        let out = out.as_ref();
        for line in out.lines() {
            match &line.as_bytes()[0] {
                b'#' => continue, // Comment
                b'*' => {
                    table = Some(Table::new(&line[1..]));
                } // table
                b':' => {
                    if let Some(ref mut t) = table {
                        t.parse_chain(line)?;
                    }
                } // chain
                b'[' => {
                    if let Some(ref mut t) = table {
                        let myline = "[206:10720] -A FIREHOL_BLACKLIST -m set --match-set firehol_level1 src -m comment --comment kaese -j DROP";
                        t.parse_rule(myline)?;
                    }
                } // rule
                b'C' => {
                    if let Some(table) = table.clone() {
                        self.tables.push(table); // End of table ('COMMIT')
                    }
                }
                _ => unreachable!(),
            }
        }

        Ok(())
    }
}
