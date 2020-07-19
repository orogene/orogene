use darling::{ast, FromDeriveInput, FromField, ToTokens};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Lit, LitStr};

#[proc_macro_derive(OroCommand, attributes(oro_config))]
pub fn derive_oro_command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let cmd = OroCommand::from_derive_input(&input).unwrap();
    quote!(#cmd).into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(supports(struct_named))]
struct OroCommand {
    ident: syn::Ident,
    generics: syn::Generics,
    data: ast::Data<(), OroCommandField>,
}

#[derive(Debug, FromField)]
struct OroCommandField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
}

impl ToTokens for OroCommand {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let OroCommand { ref data, .. } = *self;
        let fields = data
            .as_ref()
            .take_struct()
            .expect(
                "Enums not supported by derive macro. Implement OroCommandLayerConfig manually.",
            )
            .fields;
        let field_defs = fields.clone().into_iter().map(|field| {
            let OroCommandField { ident, ty, .. } = field;
            let ident = ident.clone().unwrap();
            let lit_str = Lit::Str(LitStr::new(&ident.to_string(), ident.span()));
            quote! {
                if args.occurrences_of(#lit_str) == 0 {
                    if let Ok(val) = config.get_str(#lit_str) {
                        self.#ident = #ty::from_str(&val)?;
                    }
                }
            }
        });

        let ts = quote! {
            mod oro_command_layer_config {
                use super::*;

                use std::str::FromStr;

                use anyhow::Result;
                use clap::ArgMatches;
                use oro_command::{OroConfig, OroCommandLayerConfig};

                impl OroCommandLayerConfig for PingCmd {
                    fn layer_config(&mut self, args: ArgMatches, config: OroConfig) -> Result<()> {
                        #(#field_defs)*
                        Ok(())
                    }
                }
            }
        };
        tokens.extend(ts);
    }
}
