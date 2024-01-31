use virt::connect::Connect;
use virt::error::Error;

const URI: &str = "qemu:///system";

fn main() {
    if let Err(e) = run() {
        println!("{e}");
    }
}

fn run() -> Result<(), Error> {
    let conn = Connect::open(Some(URI))?;
    let doms = conn.list_all_domains(0)?;

    for dom in doms {
        let name = dom.get_name()?;
        let info = dom.get_info()?;
        let sched = dom.get_scheduler_parameters()?;
        println!("{name}:\n  {info:?}\n  {sched:?}");
    }

    Ok(())
}
