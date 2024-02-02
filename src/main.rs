#![feature(iterator_try_collect)]

use std::error::Error;

use cursive::align::HAlign;
use cursive::event::Event;
use cursive::view::{Finder, IntoBoxedView, Nameable, Resizable};
use cursive::views::{Button, LinearLayout, PaddedView, Panel, ScrollView, TextView, ViewRef};
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

    let view = LinearLayout::vertical()
        .child(
            Panel::new(ScrollView::new(
                LinearLayout::vertical().with_name("machines"),
            ))
            .title("Machines"),
        )
        .child(Button::new("Quit", Cursive::quit));

    let view = PaddedView::lrtb(1, 1, 1, 1, view);

    siv.add_layer(view);
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
        LinearLayout::horizontal()
            .child(
                TextView::empty().with_name("state").min_width(15),
            )
            .child(
                LinearLayout::vertical()
                    .child(Button::new("Start", nop).disabled().with_name("start"))
                    .child(Button::new("Stop", nop).disabled().with_name("stop")),
            ),
    )
    .title_position(HAlign::Left)
    .into_boxed_view()
}

fn update(child: Option<&mut dyn View>, vm: Machine) -> Option<()> {
    let panel: &mut Panel<LinearLayout> = child?.downcast_mut()?;
    let mut state = panel.find_name::<TextView>("state")?;
    let mut start = panel.find_name::<Button>("start")?;
    let mut stop = panel.find_name::<Button>("stop")?;

    if vm.state != 1 {
        let name = vm.name.clone();
        start.enable();
        start.set_callback(with_ud(move |v: &mut Virt| v.start(&name)));
    } else {
        start.disable();
    }

    if vm.state != 5 {
        let name = vm.name.clone();
        stop.enable();
        stop.set_callback(with_ud(move |v: &mut Virt| v.stop(&name)));
    } else {
        stop.disable();
    }

    panel.set_title(vm.name);
    state.set_content(format!("{}", vm.state));
    Some(())
}
