mod metrics;

pub(crate) use metrics::MetricsIpset;

use std::str::FromStr;

use anyhow::{Context, Result};
use regex::Regex;
use tokio::process::Command;

use std::net::Ipv4Addr;

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
    num_ips: u32,
}

#[derive(Debug)]
pub(crate) struct IpsetState {
    lists: Vec<IpsetData>,
    ignore_list_regex: Regex,
}

enum ParserState {
    OutsideList,
    InsideList,
}

fn get_prefix_length(cidr: &str) -> Option<u8> {
    let parts: Vec<&str> = cidr.split('/').collect();
    if parts.len() != 2 {
        return None; // Invalid CIDR format
    }

    let _ip = Ipv4Addr::from_str(parts[0]).ok()?; // Validate IP part
    let prefix_length: u8 = parts[1].parse().ok()?; // Parse prefix length

    if prefix_length <= 32 {
        Some(prefix_length)
    } else {
        None // Invalid prefix length
    }
}

fn calculate_usable_ip_count(prefix_length: u8) -> u32 {
    let total_ips = 2u32.pow(32 - prefix_length as u32);

    // Subtract 2 if the prefix length is less than 31 (for network and broadcast)
    if prefix_length < 31 {
        total_ips - 2
    } else {
        total_ips // /31 and /32 don't have broadcast or network address to subtract
    }
}

impl IpsetState {
    pub(crate) fn new<S: AsRef<str>>(ignore_regex: S) -> Self {
        let re = Regex::new(ignore_regex.as_ref()).unwrap();

        Self {
            lists: Vec::new(),
            ignore_list_regex: re,
        }
    }

    pub(crate) fn filter_by_regex(&mut self) {
        self.lists
            .retain(|x| !self.ignore_list_regex.is_match(&x.name));
    }

    pub(crate) async fn parse<S: AsRef<str>>(&mut self, out: S) -> Result<()> {
        let out = out.as_ref();

        let mut state = ParserState::OutsideList;

        for line in out.lines() {
            match line {
                s if s.starts_with("Name:") => {
                    if let Some((_, right)) = line.split_once(": ") {
                        self.lists.push(IpsetData {
                            name: right.to_string(),
                            entries: Vec::new(),
                            num_ips: 0,
                        });
                    } else {
                        ()
                    }
                }
                "Members:" => {
                    state = ParserState::InsideList;
                }
                "" => {
                    state = ParserState::OutsideList;
                }
                _ if matches!(state, ParserState::InsideList) => {
                    if let Some(cur) = self.lists.last_mut() {
                        cur.entries.push(line.to_string());
                    }
                }
                _ => {}
            }
        }

        for list in self.lists.iter_mut() {
            for entry in list.entries.iter() {
                let mut num_ips = 1;
                let mut parsed_entry = entry.as_str();
                if let Some((front, _)) = entry.split_once(' ') {
                    parsed_entry = front;
                }
                if let Some(pref_len) = get_prefix_length(parsed_entry) {
                    num_ips = calculate_usable_ip_count(pref_len);
                }

                list.num_ips = num_ips;
            }
        }
        Ok(())
    }
}
