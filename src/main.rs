#![feature(hash_extract_if)]
#![feature(iterator_try_collect)]
#![feature(option_zip)]
#![feature(type_alias_impl_trait)]
#![feature(unwrap_infallible)]

use std::{
    collections::HashMap,
    error::Error,
    panic::resume_unwind,
    thread,
    time::{Duration, Instant},
};

use cursive::reexports::crossbeam_channel::{
    select, unbounded, Receiver, Sender,
};
use gag::Hold;
use rusty_pool::ThreadPool;
use ui::{Action, ButtonId};
use vm::{State, Virt, VmInfo};

mod ui;
mod vm;


pub type DynErr = Box<dyn Error + Send + Sync + 'static>;

fn main() -> Result<(), DynErr> {
    let ui = ui::create();

    let worker = thread::Builder::new()
        .name("worker".into())
        .spawn(move || worker(ui.actions, ui.controller))?;

    let hold = Hold::stderr()?;
    ui.runner.run();
    drop(hold);

    worker.join().map_err(resume_unwind).into_ok()
}

fn worker(actions: ui::Actions, ui: ui::Controller) -> Result<(), DynErr> {
    let threadpool = rusty_pool::Builder::new()
        .name("start/stop".into())
        .core_size(0)
        .max_size(4)
        .build();

    let mut ws = WorkerState {
        ui,
        virt: Virt::new()?,
        threadpool,
        vms: HashMap::new(),
        failures: unbounded(),
    };

    refresh(&mut ws);
    ws.ui.connected();

    loop {
        select! {
            recv(actions) -> action => {
                if let Ok(action) = action {
                    handle_action(&mut ws, action);
                } else {
                    break;
                }
            },

            recv(ws.failures.1) -> failure => {
                handle_failure(&mut ws, failure.unwrap());
            }
        }
    }

    Ok(())
}


struct WorkerState {
    ui: ui::Controller,
    virt: Virt,
    threadpool: ThreadPool,
    vms: HashMap<String, VmState>,
    failures: (Sender<Failure>, Receiver<Failure>),
}

struct VmState {
    state: State,
    cpu: Duration,
    timestamp: Instant,
    exists: bool,
}

struct Failure {
    vm: String,
    error: String,
}

impl VmState {
    fn label(&self) -> String {
        self.state.label().into()
    }

    fn btn(&self) -> ButtonId {
        match self.state {
            State::Running => ButtonId::Stop,
            State::Shutoff => ButtonId::Start,
            _ => ButtonId::None,
        }
    }
}


fn handle_action(ws: &mut WorkerState, action: Action) {
    match action {
        Action::Refresh => refresh(ws),
        Action::Start(name) => start_stop(ws, name, Virt::start),
        Action::Stop(name) => start_stop(ws, name, Virt::stop),
    }
}

fn handle_failure(ws: &mut WorkerState, failure: Failure) {
    if let Some(vm) = ws.vms.get_mut(&failure.vm) {
        ws.ui.error(failure.error, false);
        ws.ui.set_state(failure.vm, vm.label(), vm.btn());
    }
}

fn start_stop(
    ws: &mut WorkerState,
    name: String,
    action: fn(&Virt, &str) -> Result<(), DynErr>,
) {
    let virt = ws.virt.clone();
    let failure = ws.failures.0.clone();

    ws.threadpool.execute(move || {
        if let Err(e) = action(&virt, &name) {
            let _ = failure.send(Failure {
                vm: name,
                error: e.to_string(),
            });
        }
    })
}

fn refresh(ws: &mut WorkerState) {
    let vms = match ws.virt.machines() {
        Ok(x) => x,
        Result::Err(e) => {
            ws.ui.error(e.to_string(), false);
            ws.ui.clear_vms();
            ws.vms.clear();
            return;
        }
    };

    for vm in ws.vms.values_mut() {
        vm.exists = false;
    }

    for vm in vms {
        refresh_vm(ws, vm);
    }

    for (name, _) in ws.vms.extract_if(|_, v| !v.exists) {
        ws.ui.remove_vm(name);
    }
}

fn refresh_vm(ws: &mut WorkerState, vm: VmInfo) {
    let new = VmState {
        state: vm.state,
        cpu: vm.cpu,
        timestamp: vm.timestamp,
        exists: true,
    };

    if let Some(old) = ws.vms.get_mut(&vm.name) {
        if old.state != new.state {
            ws.ui.set_state(
                vm.name.clone(),
                new.state.label().into(),
                new.btn(),
            );
        }

        if new.state == State::Running && old.state == State::Running {
            let dcpu = new.cpu.checked_sub(old.cpu);
            let dt = new.timestamp.checked_duration_since(old.timestamp);

            let cpu = if let Some((dcpu, dt)) = dcpu.zip(dt) {
                dcpu.as_secs_f64() / dt.as_secs_f64() / vm.ncpus as f64
            } else {
                f64::NAN
            };

            ws.ui.set_cpu(vm.name, cpu);
        }

        *old = new;
    } else {
        ws.ui.add_vm(vm.name.clone(), new.label(), new.btn());
        ws.vms.insert(vm.name, new);
    }
}
