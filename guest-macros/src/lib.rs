use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, parse_quote, Expr, FnArg, ForeignItem, ItemFn, ItemForeignMod, Pat,
    Signature, Stmt, Type, TypeReference,
};

fn is_atom_type(ty: &str) -> bool {
    const ATOMS: &[&str] = &[
        "bool", "char", "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64",
    ];

    ATOMS.iter().any(|&a| a == ty)
}

#[derive(Clone, Copy)]
enum WrapMode {
    Encoded,
    Managed,
}

impl WrapMode {
    fn wrap(self, ty: Type) -> Type {
        match self {
            WrapMode::Encoded => parse_quote!(scotch_guest::EncodedPtr<#ty>),
            WrapMode::Managed => parse_quote!(scotch_guest::MemoryType),
        }
    }
}

enum TypeTranslation {
    Original,
    Wrapped(Type),
}

fn translate_type(ty: Type, mode: WrapMode) -> TypeTranslation {
    match ty {
        Type::Path(ref path)
            if is_atom_type(&path.path.segments.last().unwrap().ident.to_string()) =>
        {
            TypeTranslation::Original
        }
        Type::Reference(TypeReference {
            lifetime: None,
            mutability: None,
            elem,
            ..
        }) => TypeTranslation::Wrapped(mode.wrap(*elem)),
        Type::Array(_) | Type::Tuple(_) => TypeTranslation::Wrapped(mode.wrap(ty)),
        _ => unimplemented!("Type is unsupported, consider using a reference instead."),
    }
}

#[derive(Default)]
struct HostInputTranslation {
    call_args: Vec<Expr>,
    prelude: Vec<Stmt>,
    epilogue: Vec<Stmt>,
}

fn translate_host_inputs<'a>(it: impl Iterator<Item = &'a mut FnArg>) -> HostInputTranslation {
    let mut out = HostInputTranslation::default();

    it.map(|arg| {
        if let FnArg::Typed(arg) = arg {
            arg
        } else {
            panic!("self is not allowed in host functions")
        }
    })
    .map(|arg| {
        if let Pat::Ident(name) = arg.pat.as_mut() {
            (name.ident.clone(), &mut arg.ty)
        } else {
            panic!("Invalid function argument name")
        }
    })
    .for_each(|(name, ty)| {
        if let TypeTranslation::Wrapped(new) =
            translate_type(ty.as_ref().clone(), WrapMode::Managed)
        {
            *ty = Box::new(new);
            out.prelude
                .push(parse_quote!(let #name = scotch_guest::ManagedPtr::new(#name).unwrap();));
            out.epilogue.push(parse_quote!(#name.free();));
            out.call_args.push(parse_quote!(#name.offset()));
        } else {
            out.call_args.push(parse_quote!(#name));
        }
    });

    out
}

#[proc_macro_attribute]
pub fn host_functions(_: TokenStream, input: TokenStream) -> TokenStream {
    let host_funcs = parse_macro_input!(input as ItemForeignMod);
    let funcs = host_funcs
        .items
        .into_iter()
        .map(|item| {
            // I know let_else exists but unfortunatelly it breaks the formatting.
            if let ForeignItem::Fn(func) = item {
                func
            } else {
                panic!("Only functions are allowed in host_functions block")
            }
        })
        .map(|mut func| {
            let Signature {
                ident,
                inputs,
                output,
                ..
            } = func.sig.clone();

            let sig = &mut func.sig;
            let fake_id = format_ident!("_host_{}", sig.ident);
            sig.ident = fake_id.clone();

            let HostInputTranslation {
                prelude,
                epilogue,
                call_args,
            } = translate_host_inputs(sig.inputs.iter_mut());

            quote! {
                fn #ident(#inputs) #output {
                    extern "C" {
                        #[link_name = stringify!(#ident)]
                        #sig;
                    }

                    unsafe {
                        #(#prelude)*
                        let __out = #fake_id(#(#call_args),*);
                        #(#epilogue)*

                        __out
                    }
                }
            }
        });

    let out = quote! {
        #(#funcs)*
    };
    out.into()
}

#[derive(Default)]
struct GuestInputTranslation {
    prelude: Vec<Stmt>,
}

fn translate_guest_inputs<'a>(it: impl Iterator<Item = &'a mut FnArg>) -> GuestInputTranslation {
    let mut out = GuestInputTranslation::default();

    it.map(|arg| {
        let FnArg::Typed(arg) = arg else { panic!("self is not allowed in guest functions") };
        let Pat::Ident(id) = &*arg.pat else { panic!("Invalid function declation") };
        (id.ident.clone(), &mut arg.ty)
    })
    .for_each(|(name, ty)| {
        if let TypeTranslation::Wrapped(new) = translate_type(ty.as_ref().clone(), WrapMode::Encoded) {
            out.prelude
                .push(parse_quote!(let #name: #ty = &unsafe { #name.read().expect("Guest was given invalid pointer") };));
            *ty = Box::new(new);
        };
    });

    out
}

#[proc_macro_attribute]
pub fn guest_function(_: TokenStream, input: TokenStream) -> TokenStream {
    let mut item_fn = parse_macro_input!(input as ItemFn);
    item_fn.attrs.push(parse_quote!(#[no_mangle]));
    item_fn.sig.abi = Some(parse_quote!(extern "C"));

    let GuestInputTranslation { prelude } = translate_guest_inputs(item_fn.sig.inputs.iter_mut());
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
