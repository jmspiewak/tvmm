#![feature(iterator_try_collect)]

use std::error::Error;

use cursive::event::Event;
use cursive::view::{Finder, IntoBoxedView, Nameable, Resizable};
use cursive::views::{
    Button, Dialog, DummyView, HideableView, LinearLayout, Panel, ScrollView, TextView, ViewRef,
};
use cursive::{Cursive, CursiveExt, View};

mod ui;
mod vm;
use ui::*;
use vm::*;

type DynErr = Box<dyn Error + 'static>;

fn main() -> Result<(), DynErr> {
    let virt = Virt::new()?;
    let mut siv = Cursive::new();

    siv.set_user_data(virt);
    setup_ui(&mut siv);
    siv.run();
    Ok(())
}

fn setup_ui(siv: &mut Cursive) {
    siv.set_fps(4);
    siv.set_global_callback('q', Cursive::quit);
    siv.set_global_callback(Event::Refresh, with_ud_and_then(Virt::machines, refresh));

    let view = ScrollView::new(LinearLayout::vertical().with_name("machines"));

    let dialog = Dialog::around(view)
        .title("Machines")
        .button("Quit", Cursive::quit);

    siv.add_layer(dialog);
}

fn refresh(siv: &mut Cursive, vms: Vec<Machine>) -> Result<(), DynErr> {
    let mut layout: ViewRef<LinearLayout> = siv
        .find_name("machines")
        .ok_or("refresh: 'Machines' panel is missing")?;

    for i in (vms.len()..layout.len()).rev() {
        layout.remove_child(i);
    }

    for _ in layout.len()..vms.len() {
        layout.add_child(empty_vm_panel());
    }

    for (i, vm) in vms.into_iter().enumerate() {
        update(layout.get_child_mut(i), vm).ok_or("refresh: can't update")?;
    }

    Ok(())
}

fn empty_vm_panel() -> Box<dyn View> {
    Panel::new(
        LinearLayout::vertical()
            .child(
                LinearLayout::horizontal()
                    .child(
                        TextView::empty()
                            .with_name("name")
                            .min_width(15)
                            .max_width(60),
                    )
                    .child(
                        HideableView::new(Button::new("", nop))
                            .hidden()
                            .with_name("button")
                            .fixed_width(9),
                    ),
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("").with_name("state"))
                    .child(DummyView)
                    .child(TextView::new("").with_name("cpu")),
            ),
    )
    .into_boxed_view()
}

fn update(child: Option<&mut dyn View>, vm: Machine) -> Option<()> {
    let panel: &mut Panel<LinearLayout> = child?.downcast_mut()?;
    let mut name = panel.find_name::<TextView>("name")?;
    let mut state = panel.find_name::<TextView>("state")?;
    let mut button = panel.find_name::<HideableView<Button>>("button")?;

    let state_label = vm.state.label();
    let name_changed = name.get_content().source() != &vm.name;
    let state_changed = state.get_content().source() != state_label;

    if name_changed || state_changed {
        match vm.state {
            State::Shutoff => {
                button.set_visible(true);
                button.get_inner_mut().set_label("Start");
                button.get_inner_mut().set_callback(start(vm.name.clone()));
            }
            State::Running => {
                button.set_visible(true);
                button.get_inner_mut().set_label("Stop");
                button.get_inner_mut().set_callback(stop(vm.name.clone()));
            }
            _ => {
                button.set_visible(false);
            }
        }
    }

    if name_changed {
        name.set_content(vm.name);
    }

    if state_changed {
        state.set_content(state_label);
    }

    Some(())
}

fn start(name: String) -> impl Fn(&mut Cursive) {
    with_ud(move |v: &mut Virt| v.start(&name))
}

fn stop(name: String) -> impl Fn(&mut Cursive) {
    with_ud(move |v: &mut Virt| v.stop(&name))
}
