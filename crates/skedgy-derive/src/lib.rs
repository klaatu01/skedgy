//! skedgy-derive is a procedural macro crate that provides the `task` attribute macro.
//! The `task` attribute macro is used to define a task that can be scheduled by the `skedgy` crate.
use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use syn::{parse::Parser, parse_macro_input, ItemFn};

fn is_dependency_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Dep";
        }
    }
    false
}

fn get_dependency_type(ty: &syn::Type) -> syn::Type {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if let syn::PathArguments::AngleBracketed(ref angle_bracketed) = segment.arguments {
                if let Some(syn::GenericArgument::Type(ref ty)) = angle_bracketed.args.first() {
                    return ty.clone();
                }
            }
        }
    }
    panic!("Expected a type path with generic argument");
}

fn get_struct_name_ident(attr: &TokenStream, input_fn: &syn::ItemFn) -> syn::Ident {
    let fn_name_str = input_fn.sig.ident.to_string();

    let default_struct_name = fn_name_str.to_case(Case::Pascal).to_string();

    if let Ok(args) = syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated
        .parse(attr.clone())
    {
        if args.is_empty() {
            return syn::Ident::new(&default_struct_name, proc_macro2::Span::call_site());
        }
        if args.len() > 1 {
            panic!("Expected exactly one for struct name #[task(StructName)]");
        }
        syn::Ident::new(
            &args[0].get_ident().unwrap().to_string(),
            proc_macro2::Span::call_site(),
        )
    } else {
        panic!("expected only a punctuated path");
    }
}

#[proc_macro_attribute]
pub fn task(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let found_crate =
        proc_macro_crate::crate_name("skedgy").expect("skedgy is present in `Cargo.toml`");
    let crate_name = match found_crate {
        proc_macro_crate::FoundCrate::Itself => quote::quote!(crate),
        proc_macro_crate::FoundCrate::Name(name) => {
            let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote::quote! { #name_ident }
        }
    };
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name_ident = input_fn.sig.ident.clone();
    let struct_name_ident = get_struct_name_ident(&_attr, &input_fn);

    let mut fields = Vec::new();
    let mut execute_bindings = Vec::new();
    let mut new_struct_fields = Vec::new();
    let mut dependency_store_bindings = Vec::new();
    let mut dependency_execute_args = Vec::new();
    let mut execute_args = vec![quote::quote! {&self}];

    let asyncness = match input_fn.sig.asyncness {
        Some(_) => quote::quote! {async},
        None => quote::quote! {},
    };

    for arg in input_fn.sig.inputs.iter() {
        match arg {
            syn::FnArg::Typed(pat_type) => {
                let pat = pat_type.pat.clone();
                let ty = pat_type.ty.clone();
                if is_dependency_type(&pat_type.ty) {
                    let dep_ty = get_dependency_type(&pat_type.ty);
                    execute_args.push(quote::quote! { #pat: impl Into<#ty> });
                    execute_bindings.push(quote::quote! {
                        let #pat = #pat.into();
                    });
                    dependency_store_bindings.push(quote::quote! {
                        let #pat = dep_store.get::<#dep_ty>().unwrap();
                    });
                    dependency_execute_args.push(quote::quote! {
                        #pat
                    });
                } else {
                    fields.push(quote::quote! {
                        #pat: #ty
                    });
                    execute_bindings.push(quote::quote! {
                        let #pat = &self.#pat;
                    });
                    new_struct_fields.push(quote::quote! {
                        #pat: #pat
                    });
                }
            }
            syn::FnArg::Receiver(_) => {
                panic!("Receiver arguments are not supported");
            }
        }
    }

    let block = input_fn.block.clone();

    let ret_ty = match &input_fn.sig.output {
        syn::ReturnType::Default => quote::quote! { () },
        syn::ReturnType::Type(_, ty) => quote::quote! { #ty },
    };

    quote::quote! {
        pub(crate) mod #fn_name_ident {
            use #crate_name::Task;
            use #crate_name::DependencyStore;
            use #crate_name::BoxFuture;
            use super::*;

            pub fn new(#(#fields),*) -> #struct_name_ident {
                #struct_name_ident {
                    #(#new_struct_fields),*
                }
            }

            pub struct #struct_name_ident {
                #(#fields),*
            }

            impl #struct_name_ident {
                pub fn new(#(#fields),*) -> Self {
                    Self {
                        #(#new_struct_fields),*
                    }
                }
                pub #asyncness fn execute(#(#execute_args),*) -> #ret_ty {
                    #(#execute_bindings)*
                    #block
                }
            }

            impl Task for #struct_name_ident {
                fn run(&self, dep_store: &DependencyStore) -> BoxFuture<'_, ()> {
                    #(#dependency_store_bindings)*
                    Box::pin(async move {
                        self.execute(#(#dependency_execute_args),*).await;
                    })
                }
            }
        }
    }
    .into()
}
