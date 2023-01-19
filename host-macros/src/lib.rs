use proc_macro::TokenStream;
use quote::{format_ident, quote, __private::TokenStream as TokenStream2};
use syn::{
    parse::{Parse, ParseStream, Parser},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    BareFnArg, Ident, ItemFn, Path, ReturnType, Stmt, Token, Type, TypeBareFn, TypePath,
    Visibility,
};

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

    let ident = &item_fn.sig.ident;
    let vis = &item_fn.vis;
    let args = &item_fn.sig.inputs;

    let output = &item_fn.sig.output;
    let block = &item_fn.block;

    let out = quote! {
        #vis fn #ident(#[allow(non_snake_case)] ENV: #env_type, #args) #output {
            let __output = #block;

            __output
        }
    };

    out.into()
}

#[derive(Debug)]
#[allow(dead_code)]
struct GuestFunction {
    vis: Visibility,
    name: Ident,
    arr: Token![:],
    rename: Option<Ident>,
    ty: TypeBareFn,
}

impl Parse for GuestFunction {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let vis = input.parse()?;
        let name = input.parse()?;

        let peeker = input.lookahead1();
        let rename = if peeker.peek(Token![as]) {
            let _: Token![as] = input.parse()?;
            Some(input.parse()?)
        } else {
            None
        };

        let arr = input.parse()?;
        let ty = input.parse()?;

        Ok(Self {
            rename,
            vis,
            name,
            arr,
            ty,
        })
    }
}

#[derive(Debug, Default)]
struct TypeData {
    callback_types: Vec<Type>,
    callback_args: Vec<BareFnArg>,
    dispatch_types: Vec<Type>,
    pre_dispatch: Vec<Stmt>,
    post_dispatch: Vec<Stmt>,
}

impl TypeData {
    fn prepare(args: impl Iterator<Item = BareFnArg>) -> Self {
        let mut out = Self::default();

        args.into_iter().enumerate().for_each(|(i, arg)| {
            out.callback_types.push(arg.ty.clone());
            let name = arg.name.map(|(i, _)| i).unwrap_or_else(|| format_ident!("arg{i}"));
            let ty = &arg.ty;
            out.callback_args.push(parse_quote!(#name: #ty));

            let (disp_ty, foreign) = get_dispatch_type(arg.ty);
            if foreign {
                out.pre_dispatch = parse_quote!(let #name: #disp_ty = scotch_host::EncodedPtr::new_in(#name, &*alloc, &__view).unwrap();); 
                out.post_dispatch = parse_quote!(#name.free_in(&*alloc););
            }

            out.dispatch_types.push(disp_ty);
        });

        out
    }
}

fn is_atom_type(ty: &str) -> bool {
    const ATOMS: &[&str] = &[
        "bool", "char",
        "u8", "u16", "u32", "u64",
        "i8", "i16", "i32", "i64",
    ];

    ATOMS.iter().any(|&a| a == ty)
}

fn get_dispatch_type(ty: Type) -> (Type, bool) {
    match ty {
        Type::Path(ref path) if path.path.segments.len() == 1 &&
            is_atom_type(&path.path.segments.first().unwrap().ident.to_string()) => (ty, false),
        _ => (parse_quote!(scotch_host::EncodedPtr<#ty>), true),
    }
}

impl GuestFunction {
    fn into_handle(mut self) -> TokenStream2 {
        let (callback_return_type, dispatch_return_type): (Type, TypePath) = if let ReturnType::Type(_, ref mut ty) = self.ty.output {
            let Type::Path(ty) = ty.as_mut() else { panic!("Bad return type"); };

            (parse_quote!(Result<#ty, scotch_host::RuntimeError>), ty.clone())
        } else {
            (parse_quote!(Result<(), scotch_host::RuntimeError>), parse_quote!(()))
        };

        let export_ident = &self.name;
        let handle_ident = self.rename.unwrap_or_else(|| self.name.clone());
        let vis = self.vis;

        let TypeData {
            callback_types,
            callback_args,
            pre_dispatch,
            post_dispatch,
            dispatch_types,
        } = TypeData::prepare(self.ty.inputs.clone().into_iter());

        let dispatch_types = if dispatch_types.len() == 1 {
            quote!(#(#dispatch_types)*)
        } else {
            quote!((#(#dispatch_types),*))
        };

        let arg_names = callback_args
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                arg.name.clone().map(|(i, _)| i).unwrap_or_else(|| format_ident!("arg{i}"))
            });

        quote! {
            #[allow(non_camel_case_types)]
            #vis struct #handle_ident;
            unsafe impl scotch_host::GuestFunctionHandle for #handle_ident {
                type Callback = Box<dyn Fn(#(#callback_types),*) -> #callback_return_type>;
            }

            unsafe impl scotch_host::GuestFunctionCreator for #handle_ident {
                fn create(
                    &self,
                    store: scotch_host::StoreRef,
                    alloc: scotch_host::WasmAllocRef,
                    instance: scotch_host::InstanceRef,
                    exports: &scotch_host::Exports,
                ) -> (std::any::TypeId, scotch_host::CallbackRef) {
                    let typed_fn: scotch_host::TypedFunction<#dispatch_types, #dispatch_return_type> = exports
                        .get_typed_function(&*store.read(), stringify!(#export_ident))
                        .unwrap();

                    let callback = Box::new(move |#(#callback_args),*| {
                        let __view = instance.exports.get_memory("memory").unwrap().view(&*store.read());

                        #(#pre_dispatch)*
                        let out = typed_fn.call(&mut *store.write(), #(#arg_names),*);
                        #(#post_dispatch)*

                        out
                    }) as <Self as scotch_host::GuestFunctionHandle>::Callback;

                    (std::any::TypeId::of::<#handle_ident>(), unsafe { std::mem::transmute(callback) })
                }
            }
        }
    }
}

#[proc_macro]
pub fn guest_functions(input: TokenStream) -> TokenStream {
    let parser = Punctuated::<GuestFunction, Token![,]>::parse_terminated;
    let guest_fns = parser
        .parse(input)
        .expect("Invalid guest_functions invokation");

    let handles = guest_fns
        .into_iter()
        .map(GuestFunction::into_handle)
        .collect::<Vec<_>>();

    let output = quote! {
        #(#handles)*
    };

    output.into()
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

        quote! {
            (stringify!(#func_ident), scotch_host::Function::new_typed_with_env(_store, _env, #item))
        }
    });

    let out = quote! {
        |_store, _env| {
            scotch_host::create_imports_from_functions([ #(#tuples),* ])
        }
    };

    out.into()
}

#[proc_macro]
pub fn make_exports(input: TokenStream) -> TokenStream {
    let parser = Punctuated::<Path, Token![,]>::parse_terminated;
    let exported_fns = parser
        .parse(input)
        .expect("Invalid make_exports invokation. Expected list of paths");

    let boxes = exported_fns.into_iter().map(|item| {
        assert!(!item.segments.is_empty(), "Empty segments are not allowed");

        quote! {
            Box::new(#item) as Box<dyn scotch_host::GuestFunctionCreator>
        }
    });

    let out = quote! {
        vec![#(#boxes),*]
    };

    out.into()
}
