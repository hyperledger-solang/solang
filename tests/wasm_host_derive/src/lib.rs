use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{punctuated::Punctuated, token::Comma, Ident, ImplItem, ItemImpl, Type};

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

        // The first 2 inputs of the impl function do not belong to the wasm host function
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

    fn to_tokens(&self, host_ty: &Box<Type>) -> TokenStream2 {
        let block = &self.block;
        let params = &self.params;
        let name = &self.name;
        let module = &self.module;

        quote!(
            linker
                .define(#module, #name, ::wasmi::Func::wrap(
                    &mut store, |mut __ctx__: ::wasmi::Caller<#host_ty>, #params| {
                        let mem = __ctx__.data().memory.unwrap();
                        let (mem, vm) = mem.data_and_store_mut(&mut __ctx__);
                        #block
                    }
                ))
                .unwrap();
        )
    }
}

#[proc_macro_attribute]
pub fn wasm_host(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = syn::parse_macro_input!(item as ItemImpl);
    let host_ty = item.self_ty; // Ignoring generics, we don't need them
    let impls = item
        .items
        .iter()
        .filter_map(HostFn::new)
        .map(|f| f.to_tokens(&host_ty));

    quote!(impl #host_ty {
        fn define(
            mut store: &mut ::wasmi::Store<#host_ty>,
            linker: &mut ::wasmi::Linker<#host_ty>
        ) {
            #( #impls )*
        }
    })
    .into()
}
