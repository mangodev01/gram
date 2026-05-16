use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Attribute, Ident, Token, Type, Visibility,
};

struct SettingsInput {
    fields: Vec<Field>,
}

struct Field {
    attrs: Vec<Attribute>,
    vis: Visibility,
    name: Ident,
    ty: Type,
    default_fn: Option<Ident>,
}

impl Parse for SettingsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut fields = Vec::new();

        while !input.is_empty() {
            let attrs = input.call(Attribute::parse_outer)?;
            let vis: Visibility = input.parse()?;
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let ty: Type = input.parse()?;

            let mut default_fn = None;

            if input.peek(Token![=]) {
                input.parse::<Token![=]>()?;
                default_fn = Some(input.parse()?);
            }

            let _ = input.parse::<Token![,]>();

            fields.push(Field {
                attrs,
                vis,
                name,
                ty,
                default_fn,
            });
        }

        Ok(Self { fields })
    }
}

#[proc_macro]
pub fn settings(input: TokenStream) -> TokenStream {
    let SettingsInput { fields } = parse_macro_input!(input as SettingsInput);

    let struct_fields = fields.iter().map(|f| {
        let attrs = &f.attrs;
        let vis = &f.vis;
        let name = &f.name;
        let ty = &f.ty;

        quote! {
            #(#attrs)*
            #vis #name: #ty
        }
    });

    let defaults = fields.iter().map(|f| {
        let name = &f.name;

        if let Some(def) = &f.default_fn {
            quote! {
                #name: #def()
            }
        } else {
            quote! {
                #name: Default::default()
            }
        }
    });

    let set_arms = fields.iter().map(|f| {
        let key = f.name.to_string();
        let name = &f.name;
        let ty = &f.ty;

        quote! {
            #key => {
                let de = toml::de::ValueDeserializer::parse(value)
                    .map_err(|_| "invalid value")?;
                let parsed: #ty = serde::Deserialize::deserialize(de)
                    .map_err(|_| "invalid value")?;
                self.#name = parsed;
                Ok(())
            }
        }
    });

    let get_arms = fields.iter().map(|f| {
        let key = f.name.to_string();
        let name = &f.name;

        quote! {
            #key => Ok(format!("{:?}", self.#name))
        }
    });

    let iter_pairs_1 = fields.iter().map(|f| {
        let key = f.name.to_string();
        let name = &f.name;

        quote! {
            (#key, format!("{:?}", self.#name))
        }
    });

    let iter_pairs_2 = fields.iter().map(|f| {
        let key = f.name.to_string();
        let name = &f.name;

        quote! {
            (#key, format!("{:?}", self.#name))
        }
    });

    let expanded = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct GramSettings {
            #(#struct_fields,)*
        }

        impl GramSettings {
            pub fn new() -> Self {
                Self {
                    #(#defaults,)*
                }
            }

            pub fn set(&mut self, key: &str, value: &str) -> Result<(), &'static str> {
                match key {
                    #(#set_arms,)*
                    _ => Err("unknown setting"),
                }
            }

            pub fn get(&self, key: &str) -> Result<String, &'static str> {
                match key {
                    #(#get_arms,)*
                    _ => Err("unknown setting"),
                }
            }

            pub fn iter(&self) -> impl Iterator<Item = (&'static str, String)> + '_ {
                vec![
                    #(#iter_pairs_1,)*
                ]
                .into_iter()
            }
        }

        impl<'a> IntoIterator for &'a GramSettings {
            type Item = (&'a str, String);
            type IntoIter = std::vec::IntoIter<Self::Item>;

            fn into_iter(self) -> Self::IntoIter {
                vec![
                    #(#iter_pairs_2,)*
                ]
                .into_iter()
            }
        }
    };

    TokenStream::from(expanded)
}
