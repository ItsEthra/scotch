use proc_macro::{TokenStream, TokenTree};
/* ?? */
use quote::{__private::TokenStream as TokenStream2, format_ident, quote};
use syn::{
    parse::{Parse, ParseStream, Parser},
    parse_macro_input, parse_quote,
    punctuated::Punctuated,
    BareFnArg, FnArg, ForeignItem, ForeignItemFn, Ident, ItemFn, ItemForeignMod, Pat, PatType,
    Path, ReturnType, Stmt, Token, Type, TypeBareFn, TypeReference, Visibility,
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
            WrapMode::Encoded => parse_quote!(scotch_host::EncodedPtr<#ty>),
            WrapMode::Managed => parse_quote!(scotch_host::ManagedPtr<#ty>),
        }
    }
}

enum TypeTranslation {
    Original,
    Wrapped(Type),
}

fn translate_type(ty: Type, mode: WrapMode, allow_owned: bool) -> TypeTranslation {
    match ty {
        Type::Path(path) if is_atom_type(&path.path.segments.last().unwrap().ident.to_string()) => {
            TypeTranslation::Original
        }
        Type::Reference(TypeReference {
            lifetime: None,
            mutability: None,
            elem,
            ..
        }) => TypeTranslation::Wrapped(mode.wrap(*elem)),
        Type::Array(_) | Type::Tuple(_) => TypeTranslation::Wrapped(mode.wrap(ty)),
        Type::Path(_) if allow_owned => TypeTranslation::Wrapped(mode.wrap(ty)),
        _ => unimplemented!("Type is unsupported, consider using a reference instead."),
    }
}

#[derive(Default)]
struct HostInputTranslation {
    prelude: Vec<Stmt>,
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
        if let Pat::Ident(id) = arg.pat.as_mut() {
            (&id.ident, &mut arg.ty)
        } else {
            panic!("Invalid host function argument name")
        }
    })
    .for_each(|(name, ty)| {
        if let TypeTranslation::Wrapped(new) =
            translate_type(ty.as_ref().clone(), WrapMode::Managed, false)
        {
            out.prelude
                .push(parse_quote!(let #name: #ty = &#name.read(&__view).unwrap().0;));
            *ty.as_mut() = new;
        }
    });

    out
}

fn translate_host_output(ret: &mut ReturnType) -> Stmt {
    let mut out = parse_quote!(return out;);

    if let ReturnType::Type(_, ty) = ret {
        if let TypeTranslation::Wrapped(new) =
            translate_type(ty.as_ref().clone(), WrapMode::Managed, true)
        {
            *ty = Box::new(new);
            out = parse_quote!(return scotch_host::EncodedPtr::new_in(&out, &mut __env, &*__instance).unwrap().to_managed(););
        }
    }

    out
}

/// Macro used to annotate host function that can be exposed to guest.
/// ```ignore
/// #[host_function]
/// fn print(text: &String) {
///     println!("Wasm: {text}");
/// }
/// ```
#[proc_macro_attribute]
pub fn host_function(args: TokenStream, input: TokenStream) -> TokenStream {
    let env_type = if args.is_empty() {
        quote!(scotch_host::FunctionEnvMut<scotch_host::WasmEnv<()>>)
    } else {
        let path = parse_macro_input!(args as Path);
        quote!(scotch_host::FunctionEnvMut<scotch_host::WasmEnv<#path>>)
    };

    let mut item_fn = parse_macro_input!(input as ItemFn);
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

    let HostInputTranslation { prelude } = translate_host_inputs(item_fn.sig.inputs.iter_mut());

    let args = &item_fn.sig.inputs;

    let original_output = item_fn.sig.output.clone();
    let block = &item_fn.block;

    let epilogue = translate_host_output(&mut item_fn.sig.output);
    let output = &item_fn.sig.output;

    let out = quote! {
        #vis fn #ident(mut __env: #env_type, #args) #output {
            let __instance = __env.data().instance.upgrade().unwrap();
            let __view = __instance.exports.get_memory("memory").expect("Memory is missing").view(&__env);

            let state = &mut __env.data_mut().state;

            #(#prelude)*
            let out = (move || #original_output #block)();
            #epilogue
        }
    };

    out.into()
}

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

#[derive(Default)]
struct HandleGenerationData {
    callback_types: Vec<Type>,
    callback_args: Vec<BareFnArg>,
    dispatch_types: Vec<Type>,
    pre_dispatch: Vec<Stmt>,
    post_dispatch: Vec<Stmt>,
}

fn prepare_handle_gen_data(args: impl Iterator<Item = PatType>) -> HandleGenerationData {
    let mut out = HandleGenerationData::default();

    args.into_iter().for_each(|arg| {
        out.callback_types.push(arg.ty.as_ref().clone());
        let name = if let Pat::Ident(name) = *arg.pat {
            name.ident
        } else {
            panic!("Invalid function parameter");
        };
        let ty = &arg.ty;
        out.callback_args.push(parse_quote!(#name: #ty));

        match translate_type(arg.ty.as_ref().clone(), WrapMode::Encoded, false) {
            TypeTranslation::Wrapped(new) => {
                let pre = parse_quote! {
                    let #name: #new = scotch_host::EncodedPtr::new_in(#name, &mut *store.write(), &*instance).expect("Alloc failed");
                };
                let post = parse_quote! {
                    #name.free_in(&mut *store.write(), &*instance).expect("Free failed");
                };

                out.pre_dispatch.push(pre);
                out.post_dispatch.push(post);
                out.dispatch_types.push(new);
            }
            TypeTranslation::Original => out.dispatch_types.push(*arg.ty),
        }
    });

    out
}

fn handle_from_function(mut func: ForeignItemFn) -> TokenStream2 {
    let mut ending: Stmt = parse_quote!(return out;);
    let (callback_return_type, dispatch_return_type): (Type, Type) =
        if let ReturnType::Type(_, ref mut ty) = func.sig.output {
            let out_ty = if let TypeTranslation::Wrapped(new) =
                translate_type(ty.as_ref().clone(), WrapMode::Managed, true)
            {
                ending = parse_quote! {
                    return out.map(|ptr| {
                        let out = ptr.read(
                            &instance.exports
                                .get_memory("memory")
                                .expect("Memory is missing")
                                .view(&*store.read())
                        ).map_err(|e| scotch_host::RuntimeError::new(e.to_string()));
                        if let Ok((_, len)) = out {
                            // TODO: Should be handled somehow?
                            _ = ptr.free_in(len, &mut *store.write(), &*instance);
                        }

                        out.map(|(val, _)| val)
                    }).and_then(|x| x);
                };
                new
            } else {
                ty.as_ref().clone()
            };

            (parse_quote!(Result<#ty, scotch_host::RuntimeError>), out_ty)
        } else {
            (
                parse_quote!(Result<(), scotch_host::RuntimeError>),
                parse_quote!(()),
            )
        };

    let export_ident = &func.sig.ident;

    // damn this is nasty
    let rename = func
        .attrs
        .iter()
        .find(|attr| {
            attr.path.segments.len() == 1
                && attr.path.segments.first().unwrap().ident == "link_name"
        })
        .and_then(|attr| {
            let tokens = <TokenStream as From<_>>::from(quote!(#attr))
                .into_iter()
                .collect::<Vec<_>>();
            if tokens.len() != 2 {
                return None;
            }

            if let TokenTree::Group(g) = &tokens[1] {
                let tokens: Vec<_> = g.stream().into_iter().collect();
                if let TokenTree::Ident(id) = &tokens[0] {
                    if id.to_string() == "link_name" {
                        if let TokenTree::Literal(lit) = &tokens[2] {
                            let s = lit.to_string();
                            return Some(format_ident!("{}", s[1..][..s.len() - 2].to_owned()));
                        }
                    }
                }
            }

            None
        });

    let handle_ident = rename.unwrap_or_else(|| export_ident.clone());
    let vis = func.vis;

    let HandleGenerationData {
        callback_types,
        callback_args,
        pre_dispatch,
        post_dispatch,
        dispatch_types,
    } = prepare_handle_gen_data(func.sig.inputs.clone().into_iter().map(|arg| {
        if let FnArg::Typed(arg) = arg {
            arg
        } else {
            panic!("self is not supported in guest functions.")
        }
    }));

    let dispatch_types = if dispatch_types.len() == 1 {
        quote!(#(#dispatch_types)*)
    } else {
        quote!((#(#dispatch_types),*))
    };

    let arg_names = callback_args.iter().enumerate().map(|(i, arg)| {
        arg.name
            .clone()
            .map(|(i, _)| i)
            .unwrap_or_else(|| format_ident!("arg{i}"))
    });

    quote! {
        #[allow(non_camel_case_types)]
        #vis struct #handle_ident;
        unsafe impl scotch_host::GuestFunctionHandle for #handle_ident {
            type Callback = Box<dyn Fn(#(#callback_types),*) -> #callback_return_type>;
        }

        unsafe impl scotch_host::GuestFunctionCreator for #handle_ident {
            #[inline(always)]
            fn new() -> Self {
                Self
            }

            fn create(
                &self,
                store: scotch_host::StoreRef,
                instance: scotch_host::InstanceRef,
            ) -> Option<(std::any::TypeId, scotch_host::CallbackRef)> {
                let typed_fn: scotch_host::TypedFunction<#dispatch_types, #dispatch_return_type> = instance.exports
                    .get_typed_function(&*store.read(), stringify!(#export_ident))
                    .unwrap();

                let callback = Box::new(move |#(#callback_args),*| {
                    #(#pre_dispatch)*
                    let out = typed_fn.call(&mut *store.write(), #(#arg_names),*);
                    #(#post_dispatch)*

                    #ending
                }) as <Self as scotch_host::GuestFunctionHandle>::Callback;

                let any = Box::new(callback) as Box<dyn core::any::Any>;

                Some((std::any::TypeId::of::<#handle_ident>(), any))
            }
        }
    }
}

/// Macro that is used to create handles for guest functions.
/// ```ignore
/// guest_functions! {
///     pub add_up_list: fn(nums: &Vec<i32>) -> i32;
/// }
/// ```
#[proc_macro_attribute]
pub fn guest_functions(_: TokenStream, input: TokenStream) -> TokenStream {
    let handles = parse_macro_input!(input as ItemForeignMod)
        .items
        .into_iter()
        .map(|item| {
            if let ForeignItem::Fn(func) = item {
                handle_from_function(func)
            } else {
                panic!("Only functions are supported")
            }
        });

    let output = quote! {
        #(#handles)*
    };

    output.into()
}

/// Macro to create guest imports for `WasmPluginBuilder`.
/// ```ignore
/// #[host_function]
/// fn print(text: &String) {
///     println!("Wasm: {text}");
/// }
///
/// let plugin = WasmPluginBuilder::new()
///     .with_state(())
///     .with_imports(make_imports![print]);
/// ```
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

/// Macro to create guest exports for `WasmPluginBuilder`.
/// ```ignore
/// guest_functions! {
///     pub add_up_list: fn(nums: &Vec<i32>) -> i32;
/// }
///
/// let plugin = WasmPluginBuilder::new()
///     .with_state(())
///     .with_exports(make_exports![add_up_list]);
/// ```
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
