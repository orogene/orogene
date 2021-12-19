use quote::quote;
use syn::{parse_macro_input, DeriveInput};

use config_layer::OroConfigLayer;

mod config_layer;

#[proc_macro_derive(OroConfigLayer, attributes(config_layer))]
pub fn derive_oro_command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let cmd = match OroConfigLayer::from_derive_input(input) {
        Ok(cmd) => cmd.gen(),
        Err(err) => return err.to_compile_error().into(),
    };
    quote!(#cmd).into()
}
