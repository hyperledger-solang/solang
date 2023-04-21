use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, token::Comma, Ident, ImplItem, ItemImpl};

#[derive(Debug)]
struct HostFn {
    name: String,
    module: String,
    params: TokenStream2,
    block: TokenStream2,
}

impl HostFn {
    fn new(item: &ImplItem) -> Option<Self> {
        let item = match item {
            ImplItem::Fn(item) => item,
            _ => return None, // Only care about functions
        };

        // Only care about functions annoated with the "link" attribute
        let module = item
            .attrs
            .iter()
            .find(|attr| attr.path().get_ident().unwrap() == "link")
            .map(|attr| attr.parse_args::<Ident>().unwrap().to_string())?;

        // Collect the actual inputs of the host function
        let params = item
            .sig
            .inputs
            .iter()
            .skip(2)
            .collect::<Punctuated<_, Comma>>()
            .into_token_stream();

        Some(HostFn {
            name: item.sig.ident.to_string(),
            module,
            params,
            block: item.block.clone().into_token_stream(),
        })
    }
}

#[proc_macro_attribute]
pub fn wasm_host(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as ItemImpl);
    let ty = item.self_ty; // Ignoring generics, we don't need them
    let host_functions = item
        .items
        .iter()
        .filter_map(HostFn::new)
        .map(|f| dbg!(f))
        .collect::<Vec<_>>();

    quote!(impl #ty {
        fn define(
            store: &mut ::wasmi::Store<#ty>,
            linker: &mut ::wasmi::Linker<#ty>
        ) -> Result<(),::wasmi::errors::LinkerError> {
            Ok(())
        }
    })
    .into()
}
