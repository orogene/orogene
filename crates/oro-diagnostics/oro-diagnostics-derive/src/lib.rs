use proc_macro::TokenStream;
use quote::quote;
use syn::Data;

#[proc_macro_derive(Diagnostic, attributes(advice, category, label, ask))]
pub fn diagnostics_macro_derive(input: TokenStream) -> TokenStream {
    let ast = syn::parse(input).unwrap();

    impl_diagnostics_macro(ast)
}

fn impl_diagnostics_macro(ast: syn::DeriveInput) -> TokenStream {
    let name = ast.ident;

    match ast.data {
        Data::Enum(enm) => {
            let variants = enm.variants;

            let cat_arms = variants.iter().map(|variant| {
                let id = &variant.ident;

                let cat = variant.attrs.iter().find_map(|a| {
                    if a.path.is_ident("category") {
                        let id: syn::Ident = a.parse_args().unwrap();
                        Some(id)
                    } else {
                        None
                    }
                });

                let has_ask_attr: Vec<bool> = variant
                    .fields
                    .iter()
                    .map(|field| field.attrs.iter().any(|attr| attr.path.is_ident("ask")))
                    .collect();
                let should_ask = has_ask_attr.contains(&true);

                match variant.fields {
                    syn::Fields::Unit => cat.map(|c| {
                        quote! {
                            #id => DiagnosticCategory::#c,
                        }
                    }),
                    syn::Fields::Named(_) => cat.map(|c| {
                        quote! {
                            #id {..} => DiagnosticCategory::#c,
                        }
                    }),
                    syn::Fields::Unnamed(_) => {
                        if should_ask {
                            return Some(quote! {
                                #id(err) => err.category(),
                            });
                        }

                        cat.map(|c| {
                            quote! {
                                #id(..) => DiagnosticCategory::#c,
                            }
                        })
                    }
                }
            });

            let label_arms = variants.iter().map(|variant| {
                let id = &variant.ident;

                let labels = variant.attrs.iter().find_map(|a| {
                    if a.path.is_ident("label") {
                        let string: syn::LitStr = a.parse_args().unwrap();
                        Some(string.value())
                    } else {
                        None
                    }
                });

                let has_ask_attr: Vec<bool> = variant
                    .fields
                    .iter()
                    .map(|field| field.attrs.iter().any(|attr| attr.path.is_ident("ask")))
                    .collect();
                let should_ask = has_ask_attr.contains(&true);

                match variant.fields {
                    syn::Fields::Unit => labels.map(|l| {
                        quote! {
                            #id => #l.into(),
                        }
                    }),
                    syn::Fields::Named(_) => labels.map(|l| {
                        quote! {
                            #id {..} => #l.into(),
                        }
                    }),
                    syn::Fields::Unnamed(_) => {
                        if should_ask {
                            return Some(quote! {
                                #id(err) => err.label(),
                            });
                        }

                        labels.map(|l| {
                            quote! {
                                #id(..) => #l.into(),
                            }
                        })
                    }
                }
            });

            let advice_arms = variants.iter().map(|variant| {
                let id = &variant.ident;

                let advices = variant.attrs.iter().find_map(|a| {
                    if a.path.is_ident("advice") {
                        let string: syn::LitStr = a.parse_args().unwrap();
                        Some(string.value())
                    } else {
                        None
                    }
                });

                let has_ask_attr: Vec<bool> = variant
                    .fields
                    .iter()
                    .map(|field| field.attrs.iter().any(|attr| attr.path.is_ident("ask")))
                    .collect();
                let should_ask = has_ask_attr.contains(&true);

                match variant.fields {
                    syn::Fields::Unit => advices.map(|a| {
                        quote! {
                            #id => Some(#a.into()),
                        }
                    }),
                    syn::Fields::Named(_) => advices.map(|a| {
                        quote! {
                            #id {..} => Some(#a.into()),
                        }
                    }),
                    syn::Fields::Unnamed(_) => {
                        if should_ask {
                            return Some(quote! {
                                #id(err) => err.advice(),
                            });
                        };

                        advices.map(|a| {
                            quote! {
                                #id(..) => Some(#a.into()),
                            }
                        })
                    }
                }
            });

            let gen = quote! {
                impl Diagnostic for #name {
                    fn category(&self) -> DiagnosticCategory {
                        use #name::*;
                        match self {
                             #(#cat_arms)*
                            _ => DiagnosticCategory::Misc
                        }
                    }

                    fn label(&self) -> String {
                        use #name::*;
                        match self {
                            #(#label_arms)*
                            _ => "crate::label".into()
                        }
                    }

                    fn advice(&self) -> Option<String> {
                        use #name::*;
                        match self {
                            #(#advice_arms)*
                            _ => None
                        }
                    }
                }
            };

            gen.into()
        }
        Data::Struct(_) => {
            let label = ast
                .attrs
                .iter()
                .find_map(|a| {
                    if a.path.is_ident("label") {
                        let string: syn::LitStr = a.parse_args().unwrap();
                        Some(string.value())
                    } else {
                        None
                    }
                })
                .map_or(
                    quote! {
                        "crate::label".into()
                    },
                    |label| {
                        quote! {
                            #label.into()
                        }
                    },
                );

            let advice = ast
                .attrs
                .iter()
                .find_map(|a| {
                    if a.path.is_ident("advice") {
                        let string: syn::LitStr = a.parse_args().unwrap();
                        Some(string.value())
                    } else {
                        None
                    }
                })
                .map_or(
                    quote! {
                        None
                    },
                    |val| {
                        quote! {
                            Some(#val.into())
                        }
                    },
                );

            let cat = ast
                .attrs
                .iter()
                .find_map(|a| {
                    if a.path.is_ident("category") {
                        let string: syn::Ident = a.parse_args().unwrap();
                        Some(string)
                    } else {
                        None
                    }
                })
                .map_or(
                    quote! {
                        DiagnosticCategory::Misc
                    },
                    |cat| {
                        quote! {
                            DiagnosticCategory::#cat
                        }
                    },
                );

            let gen = quote! {
                impl Diagnostic for #name {
                    fn category(&self) -> DiagnosticCategory {
                        #cat
                    }

                    fn label(&self) -> String {
                        #label
                    }

                    fn advice(&self) -> Option<String> {
                        #advice
                    }
                }
            };

            gen.into()
        }
        _ => todo!(),
    }
}
