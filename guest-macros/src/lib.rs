use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, FnArg, ItemFn, Pat, Stmt, Type};

#[proc_macro_attribute]
pub fn host_functions(_: TokenStream, input: TokenStream) -> TokenStream {
    input
}

fn is_atom_type(ty: &str) -> bool {
    const ATOMS: &[&str] = &[
        "bool", "char", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64",
    ];

    ATOMS.iter().any(|&a| a == ty)
}

fn get_dispatch_type(ty: Type) -> (Type, bool) {
    match ty {
        Type::Path(ref path)
            if path.path.segments.len() == 1
                && is_atom_type(&path.path.segments.first().unwrap().ident.to_string()) =>
        {
            (ty, false)
        }
        _ => (parse_quote!(scotch_guest::EncodedPtr<#ty>), true),
    }
}

#[derive(Default)]
struct InputTranslation {
    prelude: Vec<Stmt>,
}

fn translate_inputs<'a>(it: impl Iterator<Item = &'a mut FnArg>) -> InputTranslation {
    let mut out = InputTranslation::default();

    it.map(|arg| {
        let FnArg::Typed(arg) = arg else { panic!("self is not allowed in guest functions") };
        let Pat::Ident(id) = &*arg.pat else { panic!("Invalid function declation") };
        (id.ident.clone(), arg.ty.as_ref().clone(), &mut arg.ty)
    })
    .for_each(|(name, ty, old)| {
        let (wrapped, is_foreign) = get_dispatch_type(ty);
        if is_foreign {
            *old = Box::new(wrapped);
            out.prelude
                .push(parse_quote!(let #name = unsafe { #name.read().expect("Guest was given invalid pointer") };));
        }
    });

    out
}

#[proc_macro_attribute]
pub fn guest_function(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(input as ItemFn);
    item_fn.attrs.push(parse_quote!(#[no_mangle]));
    item_fn.sig.abi = Some(parse_quote!(extern "C"));

    let InputTranslation { prelude } = translate_inputs(item_fn.sig.inputs.iter_mut());
    let body = item_fn.block;

    item_fn.block = parse_quote!({
        #(#prelude)*
        let __out = #body;
        __out
    });

    let out = quote! {
        #item_fn
    };

    out.into()
}
