// SPDX-License-Identifier: Apache-2.0

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{ImplItem, ItemImpl, LitInt, Type};

struct HostFn {
    name: String,
    module: String,
    params: TokenStream2,
    block: TokenStream2,
    returns: TokenStream2,
}

impl HostFn {
    fn new(item: &ImplItem) -> Option<Self> {
        let item = match item {
            ImplItem::Fn(item) => item,
            _ => return None, // Only care about functions
        };

        let module = item
            .attrs
            .iter()
            .find(|attr| attr.path().get_ident().unwrap() == "seal")
            .map(|attr| format!("seal{}", attr.parse_args::<LitInt>().unwrap()))?;

        Some(HostFn {
            name: item.sig.ident.to_string(),
            module,
            params: item.sig.inputs.to_token_stream(),
            block: item.block.to_token_stream(),
            returns: item.sig.output.to_token_stream(),
        })
    }

    fn to_tokens(&self, host_ty: &Type) -> TokenStream2 {
        let block = &self.block;
        let params = &self.params;
        let name = &self.name;
        let module = &self.module;
        let returns = &self.returns;

        quote!(
            linker
                .define(#module, #name, ::wasmi::Func::wrap(
                    &mut store, |mut __ctx__: ::wasmi::Caller<#host_ty>, #params| #returns {
                        let mem = __ctx__.data().memory.unwrap();
                        let (mem, vm) = mem.data_and_store_mut(&mut __ctx__);
                        #block
                    }
                ))
                .unwrap();
        )
    }
}

/// Helper macro for creating wasmi host function wrappers.
/// Should be used on a dedicated impl block on the host state type.
///
/// Wraps functions with the `[seal(n)]` attribute, where n is the version number, into a wasmi host function.
/// The function signature should match exactly the signature of the closure going into [`Func::wrap`][1].
/// There will be two local variables brought into scope:
/// * `mem` for accessing the memory
/// * `vm` is a mutable reference to the host state
///
/// Additionally, a function `T::define(mut store: &mut wasmi::Store<T>, linker: &mut wasmi::Linker<T>)`
/// will be generated, which defines all host functions on the linker.
///
/// [1]: https://docs.rs/wasmi/latest/wasmi/struct.Func.html#method.wrap
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
