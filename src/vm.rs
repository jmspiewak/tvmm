use virt::connect::Connect;
use virt::domain::Domain;

use crate::DynErr;

const URI: &str = "qemu:///system";

pub struct Machine {
    pub name: String,
    pub state: u32,
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
                    state: info.state,
                })
            })
            .try_collect::<Vec<_>>()?;
    
        vms.sort_by(|x, y| x.name.cmp(&y.name));
        Ok(vms)
    }
}
