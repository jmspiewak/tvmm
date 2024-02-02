use std::collections::HashMap;
use std::fmt::Display;
use std::time::{Duration, Instant};

use virt::connect::Connect;
use virt::domain::Domain;

use crate::DynErr;

const URI: &str = "qemu:///system";

#[derive(Debug, Clone)]
pub struct Machine {
    pub name: Box<str>,
    pub state: State,
    pub cpu: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    NoState,
    Running,
    Blocked,
    Paused,
    Shutdown,
    Shutoff,
    Crashed,
    PmSuspended,
    Unknown,
}

impl State {
    pub fn label(self) -> &'static str {
        match self {
            State::NoState => "No state",
            State::Running => "Running",
            State::Blocked => "Blocked",
            State::Paused => "Paused",
            State::Shutdown => "Shutting down",
            State::Shutoff => "Off",
            State::Crashed => "Crashed",
            State::PmSuspended => "Suspended",
            State::Unknown => "Unknown",
        }
    }
}

impl Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl From<u32> for State {
    fn from(value: u32) -> Self {
        match value {
            0 => Self::NoState,
            1 => Self::Running,
            2 => Self::Blocked,
            3 => Self::Paused,
            4 => Self::Shutdown,
            5 => Self::Shutoff,
            6 => Self::Crashed,
            7 => Self::PmSuspended,
            _ => Self::Unknown,
        }
    }
}

struct LastInfo {
    timestamp: Instant,
    cpu: Duration,
}

pub struct Virt {
    conn: Connect,
    last: HashMap<Box<str>, LastInfo>,
}

impl Virt {
    pub fn new() -> Result<Virt, DynErr> {
        Ok(Virt {
            conn: Connect::open(Some(URI))?,
            last: HashMap::new(),
        })
    }

    pub fn start(&self, name: &str) -> Result<(), DynErr> {
        Domain::lookup_by_name(&self.conn, name)?.create()?;
        Ok(())
    }

    pub fn stop(&self, name: &str) -> Result<(), DynErr> {
        Domain::lookup_by_name(&self.conn, name)?.shutdown()?;
        Ok(())
    }

    pub fn machines(&mut self) -> Result<Vec<Machine>, DynErr> {
        let mut vms = self
            .conn
            .list_all_domains(0)?
            .into_iter()
            .map(|dom| {
                let name = dom.get_name()?.into_boxed_str();
                let info = dom.get_info()?;
                let state = info.state.into();
                let cpu = Duration::from_nanos(info.cpu_time);
                let now = Instant::now();

                let new_last = LastInfo {
                    timestamp: now,
                    cpu,
                };

                let cpu = if let Some(last) = self.last.get_mut(&name) {
                    let dcpu = cpu.checked_sub(last.cpu);
                    let dt = now.checked_duration_since(last.timestamp);
                    *last = new_last;

                    dcpu.zip_with(dt, |dcpu, dt| {
                        dcpu.as_secs_f64() / dt.as_secs_f64() / info.nr_virt_cpu as f64
                    })
                } else {
                    self.last.insert(name.clone(), new_last);
                    None
                };

                Ok::<_, virt::error::Error>(Machine { name, state, cpu })
            })
            .try_collect::<Vec<_>>()?;

        vms.sort_by(|x, y| x.name.cmp(&y.name));
        Ok(vms)
    }
}
