use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse::Parser, parse_macro_input, punctuated::Punctuated, Ident, ItemFn, Path, Token};

#[proc_macro_attribute]
pub fn host_function(args: TokenStream, input: TokenStream) -> TokenStream {
    let env_type = if args.is_empty() {
        quote!(scotch_host::FunctionEnvMut<()>)
    } else {
        let path = parse_macro_input!(args as Path);
        quote!(scotch_host::FunctionEnvMut<#path>)
    };

    let item_fn = parse_macro_input!(input as ItemFn);
    assert!(
        item_fn.sig.asyncness.is_none(),
        "Host function can not be async"
    );
    assert!(
        item_fn.sig.constness.is_none(),
        "Host function can not be const"
    );

    let ident = make_host_func_ident(&item_fn.sig.ident);
    let vis = &item_fn.vis;
    let args = &item_fn.sig.inputs;

    let output = &item_fn.sig.output;
    let block = &item_fn.block;

    let out = quote! {
        #vis fn #ident(__env: #env_type, #args) #output {
            let __output = #block;

            __output
        }
    };

    out.into()
}

#[proc_macro]
pub fn make_imports(input: TokenStream) -> TokenStream {
    let parser = Punctuated::<Path, Token![,]>::parse_terminated;
    let imported_fns = parser
        .parse(input)
        .expect("Invalid make_imports invokation. Expected list of paths");

    let tuples = imported_fns.into_iter().map(|item| {
        assert!(!item.segments.is_empty(), "Empty segments are not allowed");

        let func_ident = &item.segments.last().unwrap().ident;
        let mangled_ident = make_host_func_ident(func_ident);

        let mut new_path = item.clone();
        new_path.segments.last_mut().as_mut().unwrap().ident = mangled_ident;

        quote! {
            (stringify!(#func_ident), scotch_host::Function::new_typed_with_env(_store, _env, #new_path))
        }
    });

    let out = quote! {
        |_store, _env| {
            scotch_host::create_imports_from_functions([ #(#tuples),* ])
        }
    };

    out.into()
}

fn make_host_func_ident(ident: &Ident) -> Ident {
    format_ident!("__scotch_host_fn_{ident}")
}
