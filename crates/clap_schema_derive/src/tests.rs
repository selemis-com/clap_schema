use syn::{Item, ItemFn, parse_quote};

use super::{expand_item_handler, validate_handler_signature};

/// Generated handler metadata must share the handler's conditional compilation.
#[test]
fn handler_metadata_preserves_conditional_compilation() {
    let handler: ItemFn = parse_quote! {
        #[cfg(feature = "remote")]
        #[cfg_attr(docsrs, doc(cfg(feature = "remote")))]
        fn run() -> Result<String, Error> {
            unreachable!()
        }
    };

    let expanded = expand_item_handler(&handler).expect("handler expansion");
    let file: syn::File = syn::parse2(expanded).expect("expanded items");
    let functions = file
        .items
        .into_iter()
        .filter_map(|item| match item {
            Item::Fn(function) => Some(function),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(functions.len(), 2);
    let helper = &functions[1];
    assert_eq!(helper.sig.ident.to_string(), "__clap_schema_operation_run");
    assert!(helper.attrs.iter().any(|attribute| attribute.path().is_ident("cfg")));
    assert!(helper.attrs.iter().any(|attribute| attribute.path().is_ident("cfg_attr")));
}

/// Opaque output components stay invalid when nested inside a concrete result type.
#[test]
fn nested_opaque_handler_outputs_are_rejected() {
    let handler: ItemFn = parse_quote! {
        fn run() -> Result<Option<(u8, impl core::fmt::Debug)>, Error> {
            unreachable!()
        }
    };

    let error = validate_handler_signature(&handler.sig).expect_err("opaque output");
    assert!(error.to_string().contains("concrete Result<T, E> output type"));
}
