mod metrics;

pub(crate) use metrics::MetricsIpset;

use std::{result::Result as StdResult, str::FromStr};

use anyhow::{Context, Result};
use tokio::process::Command;

use crate::{cli::ScrapeTarget, error::IptablesError};

pub(crate) async fn ipset() -> Result<String> {
    let cmd = format!("ipset");

    String::from_utf8(
        Command::new(&cmd)
            .arg("list")
            .output()
            .await
            .with_context(|| format!("Failed to run {cmd}"))?
            .stdout,
    )
    .with_context(|| format!("Failed {cmd} output to valid UTF-8"))
}

#[derive(Debug)]
struct IpsetData {
    name: String,
    entries: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct IpsetState {
    lists: Vec<IpsetData>,
}

impl IpsetState {
    pub(crate) fn new() -> Self {
        Self { lists: Vec::new() }
    }

    fn add(&mut self, name: String, entries: Vec<String>) {
        self.lists.push(IpsetData { name, entries });
    }

    pub(crate) async fn parse<S: AsRef<str>>(&mut self, out: S) -> Result<()> {
        let out = out.as_ref();

        let mut state: u8 = 0;
        let mut cur: Option<&mut IpsetData> = None;

        for line in out.lines() {
            match line {
                s if s.starts_with("Name") => {
                    if let Some((_, right)) = line.split_once(": ") {
                        self.lists.push(IpsetData {
                            name: right.to_string(),
                            entries: Vec::new(),
                        });
                    } else {
                        println!("The token was not found!");
                    }
                }
                "Members:" => {
                    state = 1;
                }
                "" => {
                    state = 0;
                }
                _ => {
                    if matches!(state, 1) {
                        if let Some(cur) = self.lists.last_mut() {
                            cur.entries.push(line.to_string());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}
