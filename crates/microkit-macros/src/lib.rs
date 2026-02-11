use proc_macro::TokenStream;
use quote::quote;
use std::fs;
use std::path::PathBuf;
use syn::{parse_macro_input, Item, ItemFn, LitStr};

/// Discovers and registers all endpoint modules in a directory
///
/// This macro scans the specified directory for .rs files (excluding mod.rs),
/// parses each file to find handler functions with #[utoipa::path] attributes,
/// and automatically generates everything needed for registration
///
/// # Example
///
/// In your `endpoints/mod.rs`:
/// ```rust
/// microkit::discover_endpoints!("src/endpoints");
/// ```
///
/// Then in your main lib.rs:
/// ```rust
/// endpoints::init_endpoints(&mut service)?;
/// ```
#[proc_macro]
pub fn discover_endpoints(input: TokenStream) -> TokenStream {
    let path_lit = parse_macro_input!(input as LitStr);
    let endpoints_path = path_lit.value();

    // Get the manifest directory (the crate root)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");

    let full_path = PathBuf::from(manifest_dir).join(&endpoints_path);

    // Structure to hold endpoint information
    struct EndpointInfo {
        module_name: String,
        handlers: Vec<String>,
    }

    let mut endpoints = Vec::new();

    if full_path.exists() && full_path.is_dir() {
        match fs::read_dir(&full_path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();

                    if path.is_file() {
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name_str) = file_name.to_str() {
                                // Skip mod.rs and only process .rs files
                                if file_name_str.ends_with(".rs") && file_name_str != "mod.rs" {
                                    // Extract module name (remove .rs extension)
                                    let module_name = &file_name_str[..file_name_str.len() - 3];

                                    // Parse the file to find handler functions
                                    if let Ok(content) = fs::read_to_string(&path) {
                                        if let Ok(syntax_tree) = syn::parse_file(&content) {
                                            let mut handlers = Vec::new();

                                            for item in syntax_tree.items {
                                                if let Item::Fn(func) = item {
                                                    // Check if function has #[utoipa::path] attribute
                                                    if has_utoipa_path_attr(&func) {
                                                        handlers.push(func.sig.ident.to_string());
                                                    }
                                                }
                                            }

                                            if !handlers.is_empty() {
                                                endpoints.push(EndpointInfo {
                                                    module_name: module_name.to_string(),
                                                    handlers,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                return syn::Error::new(
                    path_lit.span(),
                    format!("Failed to read directory '{}': {}", full_path.display(), e),
                )
                .to_compile_error()
                .into();
            }
        }
    } else {
        return syn::Error::new(
            path_lit.span(),
            format!("Directory '{}' does not exist", full_path.display()),
        )
        .to_compile_error()
        .into();
    }

    // Sort for consistent output
    endpoints.sort_by(|a, b| a.module_name.cmp(&b.module_name));

    if endpoints.is_empty() {
        return syn::Error::new(
            path_lit.span(),
            format!("No endpoint modules found in '{}'", full_path.display()),
        )
        .to_compile_error()
        .into();
    }

    // Generate module declarations
    let module_idents: Vec<_> = endpoints
        .iter()
        .map(|ep| syn::Ident::new(&ep.module_name, proc_macro2::Span::call_site()))
        .collect();

    let module_decls = module_idents.iter().map(|ident| {
        quote! {
            pub mod #ident;
        }
    });

    // Generate registration calls
    let register_calls = endpoints.iter().map(|ep| {
        let module_ident = syn::Ident::new(&ep.module_name, proc_macro2::Span::call_site());
        let handler_idents: Vec<_> = ep
            .handlers
            .iter()
            .map(|h| syn::Ident::new(h, proc_macro2::Span::call_site()))
            .collect();

        quote! {
            if let Some(db) = &service.database {
                let router = ::utoipa_axum::router::OpenApiRouter::new()
                    .routes(::utoipa_axum::routes!(#(#module_ident::#handler_idents),*))
                    .with_state(db.clone());
                service.add_route(router);
            }
        }
    });

    // Generate the complete code
    let expanded = quote! {
        #(#module_decls)*

        /// Automatically registers all discovered endpoint modules
        ///
        /// This function is generated by the `discover_endpoints!` macro and will
        /// register all handler functions found in each endpoint module
        pub fn init_endpoints(
            service: &mut microkit::MicroKit
        ) -> anyhow::Result<()> {
            #(#register_calls)*
            Ok(())
        }
    };

    TokenStream::from(expanded)
}

/// Check if a function has a #[utoipa::path] attribute
fn has_utoipa_path_attr(func: &ItemFn) -> bool {
    for attr in &func.attrs {
        // Check if the attribute path matches "utoipa::path"
        if attr.path().segments.len() == 2 {
            let segments: Vec<_> = attr.path().segments.iter().collect();
            if segments[0].ident == "utoipa" && segments[1].ident == "path" {
                return true;
            }
        }
    }
    false
}

/// Registers endpoint modules with a MicroKit service
///
/// # Example
///
/// ```rust
/// let db = &service.database;
/// microkit::register_endpoints!(service, db, endpoints => [users, posts]);
/// ```
#[proc_macro]
pub fn register_endpoints(input: TokenStream) -> TokenStream {
    use syn::{
        parse::{Parse, ParseStream},
        punctuated::Punctuated,
        Ident, Token,
    };

    struct RegisterEndpointsInput {
        service: Ident,
        db: Ident,
        module: Ident,
        endpoints: Vec<Ident>,
    }

    impl Parse for RegisterEndpointsInput {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let service: Ident = input.parse()?;
            input.parse::<Token![,]>()?;
            let db: Ident = input.parse()?;
            input.parse::<Token![,]>()?;
            let module: Ident = input.parse()?;
            input.parse::<Token![=>]>()?;

            let content;
            syn::bracketed!(content in input);
            let endpoints_punct = Punctuated::<Ident, Token![,]>::parse_terminated(&content)?;
            let endpoints = endpoints_punct.into_iter().collect();

            Ok(RegisterEndpointsInput {
                service,
                db,
                module,
                endpoints,
            })
        }
    }

    let RegisterEndpointsInput {
        service,
        db,
        module,
        endpoints,
    } = parse_macro_input!(input as RegisterEndpointsInput);

    let register_calls = endpoints.iter().map(|name| {
        quote! {
            #service.add_route(#module::#name::api(&#db)?);
        }
    });

    let expanded = quote! {
        #(#register_calls)*
    };

    TokenStream::from(expanded)
}
