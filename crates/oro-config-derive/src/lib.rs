//! Derive macro crate for `oro-config`.

use darling::{ast, FromDeriveInput, FromField, ToTokens};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Lit, LitStr};

#[proc_macro_derive(OroConfigLayer, attributes(oro_config))]
pub fn derive_oro_command(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let cmd = OroConfigLayer::from_derive_input(&input).unwrap();
    quote!(#cmd).into()
}

#[derive(FromDeriveInput)]
#[darling(supports(struct_named))]
struct OroConfigLayer {
    ident: syn::Ident,
    data: ast::Data<(), OroCommandField>,
}

#[derive(Debug, FromField)]
#[darling(forward_attrs)]
struct OroCommandField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    attrs: Vec<syn::Attribute>,
}

fn inner_type_of_option(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(syn::TypePath { path, .. }) = ty {
        if let Some(p) = path.segments.iter().next() {
            // TODO: could be extended to support `Vec` too?
            if p.ident != "Option" {
                return None;
            }

            if let syn::PathArguments::AngleBracketed(ab) = &p.arguments {
                if let Some(syn::GenericArgument::Type(t)) = ab.args.first() {
                    return Some(t);
                }
            }
        }
    }
    None
}

fn oro_ignored(attr: &syn::Attribute) -> bool {
    if let Ok(syn::Meta::List(meta_list)) = attr.parse_meta() {
        if meta_list.path.get_ident().unwrap() == "oro_config" {
            if let Some(syn::NestedMeta::Meta(syn::Meta::Path(p))) = meta_list.nested.first() {
                return p.get_ident().unwrap() == "ignore";
            }
        }
    }
    false
}

fn should_be_ignored(field: &OroCommandField) -> bool {
    field.attrs.iter().any(oro_ignored)
}

impl ToTokens for OroConfigLayer {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let OroConfigLayer {
            ref data,
            ref ident,
            ..
        } = *self;
        let fields = data
            .as_ref()
            .take_struct()
            .expect(
                "Enums not supported by derive macro. Implement OroCommandLayerConfig manually.",
            )
            .fields;
        let field_defs = fields
            .clone()
            .into_iter()
            .filter(|field| !should_be_ignored(field))
            .map(|field| {
                let OroCommandField { ident, ty, .. } = field;
                let ident = ident.clone().unwrap();
                let lit_str = Lit::Str(LitStr::new(&ident.to_string(), ident.span()));

                if let Some(inner) = inner_type_of_option(ty) {
                    quote! {
                        if args.value_source(#lit_str).is_none() {
                            if let Ok(val) = config.get_string(#lit_str) {
                                self.#ident = #inner::from_str(&val).ok();
                            }
                        }
                    }
                } else {
                    quote! {
                        if args.value_source(#lit_str).is_none() {
                            if let Ok(val) = config.get_string(#lit_str) {
                                self.#ident = #ty::from_str(&val).map_err(|e| ::oro_config::OroConfigError::ConfigParseError(Box::new(e)))?;
                            }
                        }
                    }
                }
            });

        let ts = quote! {
            mod oro_command_layer_config {
                use super::*;

                use std::str::FromStr;

                impl ::oro_config::OroConfigLayer for #ident {
                    fn layer_config(&mut self, args: &::clap::ArgMatches, config: &::oro_config::OroConfig) -> ::miette::Result<()> {
                        #(#field_defs)*
                        Ok(())
                    }
                }
            }
        };
        tokens.extend(ts);
    }
}
