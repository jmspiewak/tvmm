use std::fmt::Display;

use virt::connect::Connect;
use virt::domain::Domain;

use crate::DynErr;

const URI: &str = "qemu:///system";


#[derive(Debug, Clone)]
pub struct Machine {
    pub name: String,
    pub state: State,
}

#[derive(Debug, Clone, Copy)]
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


pub struct Virt {
    conn: Connect,
}

impl Virt {
    pub fn new() -> Result<Virt, DynErr> {
        Ok(Virt {
            conn: Connect::open(Some(URI))?,
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
        let mut vms = self.conn.list_all_domains(0)?
            .into_iter()
            .map(|dom| {
                let info = dom.get_info()?;

                Ok::<Machine, virt::error::Error>(Machine {
                    name: dom.get_name()?,
                    state: info.state.into(),
                })
            })
            .try_collect::<Vec<_>>()?;
    
        vms.sort_by(|x, y| x.name.cmp(&y.name));
        Ok(vms)
    }
}
