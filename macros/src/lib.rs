use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

#[proc_macro_attribute]
pub fn host_function(_: TokenStream, input: TokenStream) -> TokenStream {
    let item_fn = parse_macro_input!(input as ItemFn);
    assert!(
        item_fn.sig.asyncness.is_none(),
        "Host function can not be async"
    );
    assert!(
        item_fn.sig.constness.is_none(),
        "Host function can not be const"
    );

    let ident = quote::format_ident!("__scotch_host_fn_{}", item_fn.sig.ident);
    let vis = &item_fn.vis;
    let args = &item_fn.sig.inputs;

    let output = &item_fn.sig.output;
    let block = &item_fn.block;

    let out = quote! {
        #vis fn #ident(#args) #output {
            let __output = #block;

            __output
        }
    };

    out.into()
}

#[proc_macro]
pub fn make_imports(_: TokenStream) -> TokenStream {
    todo!()
}
