use cursive::views::{Dialog, TextView};
use cursive::Cursive;

use crate::DynErr;

pub fn nop(_: &mut Cursive) {}

pub fn with_ud<D, F>(f: F) -> impl Fn(&mut Cursive)
where
    D: 'static,
    F: Fn(&mut D) -> Result<(), DynErr>,
{
    with_ud_and_then(f, |_, _| Ok(()))
}

pub fn with_ud_and_then<D, T, F, G>(f: F, g: G) -> impl Fn(&mut Cursive)
where
    D: 'static,
    F: Fn(&mut D) -> Result<T, DynErr>,
    G: Fn(&mut Cursive, T) -> Result<(), DynErr>,
{
    handle_error(move |siv| {
        let d = siv.user_data().ok_or("Cursive::user_data")?;
        let x = f(d)?;
        g(siv, x)
    })
}

pub fn handle_error<F>(f: F) -> impl Fn(&mut Cursive)
where
    F: Fn(&mut Cursive) -> Result<(), DynErr>,
{
    move |siv| {
        let Err(e) = f(siv)
            else { return };

        siv.add_layer(Dialog::new()
            .title("Error")
            .content(TextView::new(format!("{e}")))
            .button("Quit", Cursive::quit)
            .dismiss_button("OK")
        )
    }
}
