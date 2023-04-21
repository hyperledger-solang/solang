use proc_macro::TokenStream;
use quote::quote;

struct HostFn {
    name: String,
    returns: HostFnReturn,
    item: syn::ItemFn,
}

enum HostFnReturn {
    Unit,
    U32,
    U64,
    ReturnCode,
}

impl HostFnReturn {
    fn to_wasm_sig(&self) -> proc_macro2::TokenStream {
        let ok = match self {
            Self::Unit => quote! { () },
            Self::U32 | Self::ReturnCode => quote! { ::core::primitive::u32 },
            Self::U64 => quote! { ::core::primitive::u64 },
        };
        quote! {
            ::core::result::Result<#ok, ::wasmi::core::Trap>
        }
    }
}

impl HostFn {
    fn new(item: &syn::ImplItem) -> Option<Self> {
        let returns = HostFnReturn::Unit;
        let name = todo!();

        Some(HostFn {
            name,
            returns,
            item: todo!(),
        })
    }
}

#[proc_macro_attribute]
pub fn wasm_host(_attr: TokenStream, item: TokenStream) -> TokenStream {
    //let items = syn::parse_macro_input!(item as syn::ItemImpl).items.iter().filter();
    let item = syn::parse_macro_input!(item as syn::ItemImpl);
    let host_functions = item
        .items
        .iter()
        .filter_map(HostFn::new)
        .collect::<Vec<_>>();

    //quote!(#item).into()
    todo!()
}
