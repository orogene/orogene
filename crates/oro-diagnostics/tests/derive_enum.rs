use oro_diagnostics::Diagnostic;
use oro_diagnostics::DiagnosticCategory;
use oro_diagnostics::Explain;
use oro_diagnostics_derive::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
#[error("Rainbow error.")]
#[label("critical::rainbow")]
#[advice("Rainbow.")]
#[category(Misc)]
pub struct Rainbow;

impl Explain for Rainbow {}

#[derive(Debug, Error, Diagnostic)]
#[error("Critical error.")]
pub enum Critical {
    #[category(Misc)]
    #[label("critical::blue")]
    #[advice("Blue.")]
    Blue(String),
    #[label("critical::red")]
    #[advice("Red.")]
    #[category(Parse)]
    Red,
    #[label("critical::orange")]
    #[advice("Orange.")]
    #[category(Fs)]
    Orange,
    Transparent(#[ask] Rainbow),
}

impl Explain for Critical {}

#[test]
fn it_works() {
    let blue = Critical::Blue("test".into());
    assert_eq!("Blue.", blue.advice().unwrap());
    assert_eq!("critical::blue", blue.label());
    assert_eq!(DiagnosticCategory::Misc, blue.category());

    let red = Critical::Red;
    assert_eq!("Red.", red.advice().unwrap());
    assert_eq!("critical::red", red.label());
    assert_eq!(DiagnosticCategory::Parse, red.category());

    let orange = Critical::Orange;
    assert_eq!("Orange.", orange.advice().unwrap());
    assert_eq!("critical::orange", orange.label());
    assert_eq!(DiagnosticCategory::Fs, orange.category());

    let rainbow = Rainbow {};

    let transp = Critical::Transparent(rainbow);
    assert_eq!("Rainbow.", transp.advice().unwrap());
    assert_eq!("critical::rainbow", transp.label());
    assert_eq!(DiagnosticCategory::Misc, transp.category());
}
