use cursive::{
    event::{Event, Key},
    reexports::crossbeam_channel::{unbounded, Receiver, Sender},
    view::{Finder, Nameable, Resizable, Scrollable},
    views::{
        BoxedView, Button, Dialog, DialogFocus, DummyView, LinearLayout, Panel,
        ScreensView, TextContentRef, TextView, ViewRef,
    },
    CbSink, Cursive, CursiveExt, View,
};

type VmPanel = impl View;
const MAIN: &str = "main";
const SCREEN: &str = "screen";
const MACHINES: &str = "machines";
const NAME: &str = "name";
const STATE: &str = "state";
const CPU: &str = "cpu";
const BUTTONS: &str = "buttons";


#[derive(Clone)]
pub enum Action {
    Refresh,
    Start(String),
    Stop(String),
}

pub enum ButtonId {
    None = 0,
    Start = 1,
    Stop = 2,
}


pub type Actions = Receiver<Action>;

#[derive(Clone)]
pub struct Controller {
    callbacks: CbSink,
}

pub struct Runner {
    siv: Cursive,
}

pub struct Ui {
    pub actions: Actions,
    pub controller: Controller,
    pub runner: Runner,
}


pub fn create() -> Ui {
    let mut siv = Cursive::new();
    let (action_sender, action_receiver) = unbounded();

    siv.set_fps(2);
    siv.set_user_data(action_sender);
    siv.set_global_callback('q', Cursive::quit);
    siv.set_global_callback(Key::Esc, Cursive::quit);
    siv.set_global_callback(Event::Refresh, refresh);

    let mut view = ScreensView::single_screen(BoxedView::boxed(
        LinearLayout::vertical().with_name(MACHINES).scrollable(),
    ))
    .with_name(SCREEN);

    view.get_mut()
        .add_active_screen(BoxedView::boxed(TextView::new("Connecting...")));

    let dialog = Dialog::around(view)
        .title("Machines")
        .button("Quit", Cursive::quit)
        .with_name(MAIN)
        .fixed_width(38)
        .min_height(8);

    siv.add_layer(dialog);

    Ui {
        actions: action_receiver,
        controller: Controller {
            callbacks: siv.cb_sink().clone(),
        },
        runner: Runner { siv },
    }
}


impl Runner {
    pub fn run(mut self) {
        self.siv.run();
    }
}

impl Controller {
    pub fn error(&self, msg: String, fatal: bool) {
        self.send(move |siv| error(siv, msg, fatal));
    }

    pub fn connected(&self) {
        self.send(|siv| {
            let mut screen: ViewRef<ScreensView> =
                siv.find_name(SCREEN).unwrap();

            screen.set_active_screen(0);
        });
    }

    pub fn add_vm(&self, name: String, state: String, btn: ButtonId) {
        self.send(move |siv| {
            let vms = &mut machine_list(siv);

            let idx = 'inspoint: {
                for i in 0..vms.len() {
                    if vm_name(vm_at(vms, i).unwrap()).source() > &name {
                        break 'inspoint i;
                    }
                }

                vms.len()
            };

            vms.insert_child(idx, create_panel(name, state, btn));
            reset_focus(siv);
        });
    }

    pub fn remove_vm(&self, name: String) {
        self.send(move |siv| {
            let vms = &mut machine_list(siv);

            if let Some(i) = find_vm(vms, &name) {
                vms.remove_child(i);
            }

            reset_focus(siv);
        });
    }

    pub fn clear_vms(&self) {
        self.send(|siv| {
            machine_list(siv).clear();
            reset_focus(siv);
        });
    }

    pub fn set_state(&self, name: String, state: String, btn: ButtonId) {
        self.send(with_vm(name, move |panel| set_state(panel, state, btn)));
    }

    pub fn set_cpu(&self, name: String, cpu: f64) {
        self.send(with_vm(name, move |panel| set_cpu(panel, cpu)));
    }

    fn send(&self, cb: impl FnOnce(&mut Cursive) + Send + 'static) {
        self.callbacks
            .try_send(Box::new(cb))
            .expect("Can't send message to UI thread");
    }
}


fn reset_focus(siv: &mut Cursive) {
    siv.call_on_name(MAIN, |m: &mut Dialog| {
        m.set_focus(DialogFocus::Button(0));
    });
}

fn with_vm(
    name: String,
    f: impl FnOnce(&mut VmPanel),
) -> impl FnOnce(&mut Cursive) {
    move |siv| {
        let vms = &mut machine_list(siv);

        if let Some(i) = find_vm(vms, &name) {
            f(vm_at(vms, i).unwrap());
        } else {
            error(siv, format!("VmPanel for '{name}' not found"), false);
        }
    }
}

fn machine_list(siv: &mut Cursive) -> ViewRef<LinearLayout> {
    siv.find_name::<LinearLayout>(MACHINES).unwrap()
}

fn find_vm(vms: &mut LinearLayout, name: &str) -> Option<usize> {
    for i in 0..vms.len() {
        if vm_name(vm_at(vms, i).unwrap()).source() == name {
            return Some(i);
        }
    }

    None
}

fn vm_at(vms: &mut LinearLayout, i: usize) -> Option<&mut VmPanel> {
    vms.get_child_mut(i)
        .and_then(|v| v.downcast_mut::<VmPanel>())
}

fn vm_name(panel: &mut VmPanel) -> TextContentRef {
    panel.find_name::<TextView>(NAME).unwrap().get_content()
}


fn create_panel(name: String, state: String, btn: ButtonId) -> VmPanel {
    let start = on_click(name.clone(), Action::Start);
    let stop = on_click(name.clone(), Action::Stop);

    let mut buttons = ScreensView::single_screen(BoxedView::boxed(DummyView));
    buttons.add_screen(BoxedView::boxed(Button::new("Start", start)));
    buttons.add_screen(BoxedView::boxed(Button::new("Stop", stop)));
    buttons.set_active_screen(btn as usize);

    Panel::new(
        LinearLayout::vertical()
            .child(TextView::new(name).with_name(NAME))
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new(state).with_name(STATE))
                    .child(DummyView)
                    .child(TextView::new("").with_name(CPU))
                    .child(DummyView.full_width())
                    .child(buttons.with_name(BUTTONS)),
            ),
    )
}

fn set_state(panel: &mut VmPanel, state: String, btn: ButtonId) {
    panel
        .find_name::<TextView>(STATE)
        .unwrap()
        .set_content(state);

    panel
        .find_name::<ScreensView>(BUTTONS)
        .unwrap()
        .set_active_screen(btn as usize);

    panel.find_name::<TextView>(CPU).unwrap().set_content("");
}

fn set_cpu(panel: &mut VmPanel, cpu: f64) {
    panel
        .find_name::<TextView>(CPU)
        .unwrap()
        .set_content(format!("[{:5.1}%]", 100.0 * cpu));
}


fn refresh(siv: &mut Cursive) {
    send(siv, Action::Refresh);
}

fn on_click(
    name: String,
    action: impl Fn(String) -> Action,
) -> impl Fn(&mut Cursive) {
    move |siv| {
        let vms = &mut machine_list(siv);

        if let Some(i) = find_vm(vms, &name) {
            vm_at(vms, i)
                .unwrap()
                .find_name::<ScreensView>(BUTTONS)
                .unwrap()
                .set_active_screen(ButtonId::None as usize);
        }

        send(siv, action(name.clone()))
    }
}

fn send(siv: &mut Cursive, act: Action) {
    let actions: &mut Sender<Action> = siv.user_data().unwrap();

    if let Err(_) = actions.send(act.clone()) {
        error(siv, "Fatal error".into(), true);
    }
}

fn error(siv: &mut Cursive, msg: String, fatal: bool) {
    let mut dialog = Dialog::around(TextView::new(msg)).title("Error");

    if !fatal {
        dialog.add_button("OK", |siv| {
            siv.pop_layer();
        });
    }

    dialog.add_button("Quit", Cursive::quit);
    siv.add_layer(dialog);
}
