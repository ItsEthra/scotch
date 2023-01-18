use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{
    parse::{Parse, ParseStream, Parser},
    parse_macro_input,
    punctuated::Punctuated,
    Ident, ItemFn, Path, ReturnType, Token, Type, TypeBareFn, Visibility,
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
        #vis fn #ident(__env: #env_type, #args) #output {
            let __output = #block;

            __output
        }
    };

    out.into()
}

#[proc_macro]
pub fn guest_functions(input: TokenStream) -> TokenStream {
    #[derive(Debug)]
    #[allow(dead_code)]
    struct GuestFunction {
        vis: Visibility,
        name: Ident,
        arr: Token![=>],
        ty: TypeBareFn,
    }

    impl Parse for GuestFunction {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(Self {
                vis: input.parse()?,
                name: input.parse()?,
                arr: input.parse()?,
                ty: input.parse()?,
            })
        }
    }

    let parser = Punctuated::<GuestFunction, Token![,]>::parse_terminated;
    let guest_fns = parser
        .parse(input)
        .expect("Invalid guest_functions invokation.");

    let handles = guest_fns
        .into_iter()
        .map(|mut f| {
            if let ReturnType::Type(_, ref mut ty) = f.ty.output {
                let Type::Path(ty) = ty.as_mut() else { panic!("Bad return type"); };
                ty.path = syn::parse2(quote!(Result<#ty, scotch_host::RuntimeError>)).unwrap();
            } else {
                f.ty.output = syn::parse2(quote!(Result<(), scotch_host::RuntimeError>)).unwrap();
            };

            let export_ident = &f.name;
            let vis = f.vis;

            let arg_types = f.ty.inputs
                .iter()
                .map(|arg| arg.ty.clone())
                .collect::<Vec<_>>();

            let return_type = f.ty.output.clone();

            let typed_fn_args = if f.ty.inputs.len() == 1 {
                quote!(#(#arg_types)*)
            } else {
                quote!((#(#arg_types),*))
            };

            let arg_names = f.ty.inputs
                .into_iter()
                .enumerate()
                .map(|(i, arg)|
                    arg.name.map(|(i, _)| i).unwrap_or_else(|| format_ident!("arg{i}"))
                );
            let arg_names2 = arg_names.clone();

            quote! {
                #[allow(non_camel_case_types)]
                #vis struct #export_ident;
                unsafe impl scotch_host::GuestFunctionHandle for #export_ident {
                    type Callback = Box<dyn Fn(#(#arg_types),*) #return_type>;
                }

                unsafe impl scotch_host::GuestFunctionCreator for #export_ident {
                    fn create(
                        &self,
                        store: scotch_host::StoreRef,
                        exports: &scotch_host::Exports,
                    ) -> (std::any::TypeId, scotch_host::CallbackRef) {
                        let typed_fn: scotch_host::TypedFunction<#typed_fn_args, _> = exports
                            .get_typed_function(&*store.read(), stringify!(#export_ident))
                            .unwrap();

                        let callback = Box::new(move |#(#arg_names),*| {
                            typed_fn.call(&mut *store.write(), #(#arg_names2),*)
                        }) as <Self as scotch_host::GuestFunctionHandle>::Callback;

                        (std::any::TypeId::of::<#export_ident>(), unsafe { std::mem::transmute(callback) })
                    }
                }
            }
        })
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
