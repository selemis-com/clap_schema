//! Proc macros for `clap_schema`.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Fields, GenericArgument, Ident, ImplItemFn, ItemFn, Meta, Path,
    PathArguments, ReturnType, Token, Type, parse_macro_input,
};

/// Derives the root `clap_schema::CliSchema` implementation.
///
/// The derive reflects the root Clap parser and registers output contracts for
/// its root operation and/or subcommand tree. Invocation syntax stays owned by
/// Clap.
///
/// # `#[schema(...)]` options
///
/// - `handler = path` binds an executable root operation to its canonical handler.
/// - `metadata = Type` declares the application-wide metadata schema type. It is schema-only:
///   `clap_schema` never constructs or serializes values of `Type`.
///
/// Root `metadata` describes the application-wide metadata vocabulary, not a supplement specific
/// to an executable root handler. Builder-style applications can supplement a registered root
/// operation directly through `Operation::metadata`.
#[proc_macro_derive(CliSchema, attributes(schema, command))]
pub fn derive_cli_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_cli_schema(input).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Derives canonical-handler registration for a Clap subcommand enum.
///
/// Every contract-visible executable operation names its handler explicitly:
///
/// ```ignore
/// #[schema(handler = create)]
/// Create(CreateArgs),
/// ```
///
/// This makes output identity independent of the input carrier and supports
/// tuple, struct-style, and unit variants as well as reused `Args` types.
///
/// Normal `#[command(subcommand)]` and `#[command(flatten)]` nesting is followed
/// automatically. When an `Args` payload itself contains a subcommand field,
/// use `subcommands = Type` on the parent variant. `handler` and `subcommands`
/// may be combined for an executable parent with optional children. Executable
/// operations may also declare `metadata = Type` to supplement the root
/// application metadata schema. Metadata can only be attached to ordinary executable variants;
/// command groups, flattened variants, skipped variants, and external subcommands do not carry an
/// operation-specific metadata schema. The application owns all concrete metadata values.
#[proc_macro_derive(CommandSchema, attributes(schema, command))]
pub fn derive_command_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_command_schema(input).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Marks the canonical implementation of a contract-visible operation.
///
/// The attribute leaves runtime behavior unchanged and generates one hidden
/// companion metadata function. The declared `Result<T, E>` return type is the
/// sole source of the successful output contract; type aliases are supported.
///
/// Sync, `const fn`, async, free functions, associated functions, and inherent
/// methods with any receiver form are supported. Generic handlers are rejected
/// because they do not identify one concrete output contract.
#[proc_macro_attribute]
pub fn handler(attribute: TokenStream, input: TokenStream) -> TokenStream {
    if !attribute.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[clap_schema::handler] does not accept arguments",
        )
        .into_compile_error()
        .into();
    }

    let tokens = TokenStream2::from(input);
    if let Ok(method) = syn::parse2::<ImplItemFn>(tokens.clone())
        && matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_)))
    {
        return expand_impl_handler(&method).unwrap_or_else(syn::Error::into_compile_error).into();
    }

    let function = match syn::parse2::<ItemFn>(tokens) {
        Ok(function) => function,
        Err(error) => return error.into_compile_error().into(),
    };
    expand_item_handler(&function).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Returns handler-derived operation metadata for builder-style Clap.
///
/// The macro accepts the same handler path used by `#[schema(handler = ...)]`.
/// It has no syntax for declaring an output type manually. The returned `clap_schema::Operation`
/// can be supplemented with an application-defined metadata schema using `Operation::metadata`.
#[proc_macro]
pub fn operation(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as Path);
    match handler_helper_path(path) {
        Ok(helper) => quote!(#helper()).into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// Expands a free or associated handler and its metadata companion.
fn expand_item_handler(function: &ItemFn) -> syn::Result<TokenStream2> {
    validate_handler_signature(&function.sig)?;
    let helper = handler_helper_ident(&function.sig.ident);
    let output = declared_return_type(&function.sig)?;
    let visibility = &function.vis;
    let conditional = conditional_attributes(&function.attrs);
    let crate_path = clap_schema_path();

    Ok(quote! {
        #function

        #(#conditional)*
        #[doc(hidden)]
        #[doc = "Generated clap_schema operation metadata."]
        #visibility fn #helper() -> #crate_path::Operation {
            #crate_path::__private::operation_from_result::<#output>()
        }
    })
}

/// Expands an inherent receiver method and its metadata companion.
fn expand_impl_handler(method: &ImplItemFn) -> syn::Result<TokenStream2> {
    validate_handler_signature(&method.sig)?;
    let helper = handler_helper_ident(&method.sig.ident);
    let output = declared_return_type(&method.sig)?;
    let visibility = &method.vis;
    let conditional = conditional_attributes(&method.attrs);
    let crate_path = clap_schema_path();

    Ok(quote! {
        #method

        #(#conditional)*
        #[doc(hidden)]
        #[doc = "Generated clap_schema operation metadata."]
        #visibility fn #helper() -> #crate_path::Operation {
            #crate_path::__private::operation_from_result::<#output>()
        }
    })
}

/// Returns conditional-compilation attributes that must also guard generated metadata.
fn conditional_attributes(attrs: &[Attribute]) -> Vec<&Attribute> {
    attrs
        .iter()
        .filter(|attribute| {
            attribute.path().is_ident("cfg") || attribute.path().is_ident("cfg_attr")
        })
        .collect()
}

/// Validates handler signature properties required for one concrete output contract.
fn validate_handler_signature(signature: &syn::Signature) -> syn::Result<()> {
    if signature.unsafety.is_some()
        || signature.abi.is_some()
        || signature.variadic.is_some()
        || !signature.generics.params.is_empty()
        || signature.generics.where_clause.is_some()
    {
        return Err(syn::Error::new_spanned(
            signature,
            "clap_schema handlers use a plain non-generic function signature",
        ));
    }
    let output = declared_return_type(signature)?;
    if contains_impl_trait(output) {
        return Err(syn::Error::new_spanned(
            output,
            "clap_schema handlers require a concrete Result<T, E> output type",
        ));
    }
    Ok(())
}

/// Returns the handler's declared return type.
fn declared_return_type(signature: &syn::Signature) -> syn::Result<&Type> {
    match &signature.output {
        ReturnType::Type(_, output) => Ok(output),
        ReturnType::Default => {
            Err(syn::Error::new_spanned(signature, "clap_schema handlers must return Result<T, E>"))
        }
    }
}

/// Returns whether a type contains an opaque `impl Trait` component.
fn contains_impl_trait(ty: &Type) -> bool {
    match ty {
        Type::ImplTrait(_) => true,
        Type::Array(ty) => contains_impl_trait(&ty.elem),
        Type::Group(ty) => contains_impl_trait(&ty.elem),
        Type::Paren(ty) => contains_impl_trait(&ty.elem),
        Type::Ptr(ty) => contains_impl_trait(&ty.elem),
        Type::Reference(ty) => contains_impl_trait(&ty.elem),
        Type::Slice(ty) => contains_impl_trait(&ty.elem),
        Type::Tuple(ty) => ty.elems.iter().any(contains_impl_trait),
        Type::Path(ty) => ty.path.segments.iter().any(|segment| {
            let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
                return false;
            };
            arguments.args.iter().any(|argument| match argument {
                GenericArgument::Type(ty) => contains_impl_trait(ty),
                GenericArgument::AssocType(binding) => contains_impl_trait(&binding.ty),
                _ => false,
            })
        }),
        _ => false,
    }
}

/// Returns the generated metadata companion identifier for a handler name.
fn handler_helper_ident(handler: &Ident) -> Ident {
    format_ident!("__clap_schema_operation_{}", handler)
}

/// Rewrites a handler path to the generated metadata companion path.
fn handler_helper_path(mut path: Path) -> syn::Result<Path> {
    let Some(last) = path.segments.last_mut() else {
        return Err(syn::Error::new_spanned(path, "expected a handler path"));
    };
    if !matches!(last.arguments, PathArguments::None) {
        return Err(syn::Error::new_spanned(
            &last.arguments,
            "handler paths cannot contain generic arguments",
        ));
    }
    last.ident = handler_helper_ident(&last.ident);
    Ok(path)
}

/// Expands a `CliSchema` derive into root operation registration.
fn expand_cli_schema(input: DeriveInput) -> syn::Result<TokenStream2> {
    let crate_path = clap_schema_path();
    let RootSchema { handler, metadata } = parse_root_schema(&input.attrs)?;
    let commands = find_subcommand_field(&input)?;
    if handler.is_none() && commands.is_none() {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "CliSchema requires a root #[schema(handler = ...)] or a #[command(subcommand)] field",
        ));
    }

    let name = input.ident;
    let generics = input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let root_metadata = metadata.map(|metadata| {
        quote! {
            registry.metadata::<#metadata>();
        }
    });
    let root_handler = if let Some(handler) = handler {
        let helper = handler_helper_path(handler)?;
        Some(quote! {
            registry.operation(&[], #helper());
        })
    } else {
        None
    };
    let child_registration = commands.map(|commands| {
        quote! {
            let mut prefix = Vec::new();
            <#commands as #crate_path::CommandSchema>::__clap_schema_register(
                &mut prefix,
                registry,
            )?;
        }
    });

    Ok(quote! {
        impl #impl_generics #crate_path::CliSchema for #name #type_generics #where_clause {

            fn __clap_schema_register(
                registry: &mut #crate_path::__private::Registry,
            ) -> #crate_path::Result<()> {
                #root_metadata
                #root_handler
                #child_registration
                Ok(())
            }
        }
    })
}

/// Expands a `CommandSchema` derive into operation and child registration.
fn expand_command_schema(input: DeriveInput) -> syn::Result<TokenStream2> {
    let crate_path = clap_schema_path();
    let Data::Enum(data) = input.data else {
        return Err(syn::Error::new_spanned(
            input.ident,
            "CommandSchema can only be derived for enums",
        ));
    };

    let name = input.ident;
    let generics = input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut steps = Vec::new();

    for variant in data.variants {
        let command = parse_command_behavior(&variant.attrs)?;
        let schema = parse_variant_schema(&variant.attrs)?;
        let payload = single_payload_type(&variant.fields);

        if command.disposition != CommandDisposition::Normal {
            if !schema.is_empty() {
                return Err(syn::Error::new_spanned(
                    variant.ident,
                    "schema metadata cannot be attached to a clap-skipped or external subcommand variant",
                ));
            }
            continue;
        }

        if schema.skip && schema.has_operation_metadata() {
            return Err(syn::Error::new_spanned(
                variant.ident,
                "#[schema(skip)] cannot be combined with handler, subcommands, or metadata",
            ));
        }

        if command.nesting == CommandNesting::Flatten {
            let child = payload.ok_or_else(|| {
                syn::Error::new_spanned(
                    &variant.ident,
                    "flattened subcommands require a single tuple payload",
                )
            })?;
            if schema.has_operation_metadata() {
                return Err(syn::Error::new_spanned(
                    variant.ident,
                    "flattened subcommands cannot declare operation schema metadata",
                ));
            }
            let register = (!schema.skip).then(|| {
                quote! {
                    <#child as #crate_path::CommandSchema>::__clap_schema_register(
                        prefix,
                        registry,
                    )?;
                }
            });
            steps.push(quote! {
                {
                    #register
                    let __count =
                        <#child as #crate_path::__private::clap::Subcommand>::augment_subcommands(
                            #crate_path::__private::clap::Command::new("__clap_schema_probe")
                        )
                        .get_subcommands()
                        .count();
                    for _ in 0..__count {
                        if __clap_schema_commands.next().is_none() {
                            return Err(#crate_path::Error::DerivedCommandMismatch {
                                type_name: ::core::any::type_name::<Self>(),
                            });
                        }
                    }
                }
            });
            continue;
        }

        let consume = quote! {
            let __clap_schema_command = __clap_schema_commands
                .next()
                .ok_or_else(|| #crate_path::Error::DerivedCommandMismatch {
                    type_name: ::core::any::type_name::<Self>(),
                })?;
        };

        if schema.skip {
            steps.push(consume);
            continue;
        }

        if command.nesting == CommandNesting::Subcommand {
            let child = payload.ok_or_else(|| {
                syn::Error::new_spanned(
                    &variant.ident,
                    "nested subcommands require a single tuple payload",
                )
            })?;
            if schema.has_operation_metadata() {
                return Err(syn::Error::new_spanned(
                    variant.ident,
                    "#[command(subcommand)] groups cannot declare handler, subcommands, or metadata",
                ));
            }
            steps.push(quote! {
                {
                    #consume
                    prefix.push(__clap_schema_command.get_name().to_owned());
                    <#child as #crate_path::CommandSchema>::__clap_schema_register(
                        prefix,
                        registry,
                    )?;
                    prefix.pop();
                }
            });
            continue;
        }

        if schema.handler.is_none() && schema.subcommands.is_none() {
            return Err(syn::Error::new_spanned(
                variant.ident,
                "contract-visible executable commands require #[schema(handler = path)]; commands whose Args contain child subcommands also declare subcommands = Type",
            ));
        }
        let register_handler = if let Some(handler) = schema.handler.as_ref() {
            let helper = handler_helper_path(handler.clone())?;
            let metadata = schema.metadata.as_ref().map(|metadata| {
                quote! { .metadata::<#metadata>() }
            });
            Some(quote! {
                registry.operation(prefix, #helper() #metadata);
            })
        } else {
            None
        };
        let register_children = schema.subcommands.map(|child| {
            quote! {
                let __clap_schema_child_probe =
                    <#child as #crate_path::__private::clap::Subcommand>::augment_subcommands(
                        #crate_path::__private::clap::Command::new("__clap_schema_child_probe")
                    );
                let mut __clap_schema_expected_children =
                    __clap_schema_child_probe.get_subcommands();
                let mut __clap_schema_actual_children =
                    __clap_schema_command.get_subcommands();
                loop {
                    match (
                        __clap_schema_expected_children.next(),
                        __clap_schema_actual_children.next(),
                    ) {
                        (Some(expected), Some(actual))
                            if expected.get_name() == actual.get_name() => {}
                        (None, None) => break,
                        _ => {
                            return Err(#crate_path::Error::DerivedCommandMismatch {
                                type_name: ::core::any::type_name::<#child>(),
                            });
                        }
                    }
                }
                <#child as #crate_path::CommandSchema>::__clap_schema_register(
                    prefix,
                    registry,
                )?;
            }
        });
        let reject_unregistered_children = register_children.is_none().then(|| {
            quote! {
                if __clap_schema_command.get_subcommands().next().is_some() {
                    return Err(#crate_path::Error::UnregisteredSubcommands {
                        path: prefix.clone(),
                    });
                }
            }
        });

        steps.push(quote! {
            {
                #consume
                prefix.push(__clap_schema_command.get_name().to_owned());
                #register_handler
                #register_children
                #reject_unregistered_children
                prefix.pop();
            }
        });
    }

    Ok(quote! {
        impl #impl_generics #crate_path::CommandSchema for #name #type_generics #where_clause {
            fn __clap_schema_register(
                prefix: &mut Vec<String>,
                registry: &mut #crate_path::__private::Registry,
            ) -> #crate_path::Result<()> {
                let __clap_schema_probe =
                    <Self as #crate_path::__private::clap::Subcommand>::augment_subcommands(
                        #crate_path::__private::clap::Command::new("__clap_schema_probe")
                    );
                let mut __clap_schema_commands = __clap_schema_probe.get_subcommands();
                #(#steps)*
                if __clap_schema_commands.next().is_some() {
                    return Err(#crate_path::Error::DerivedCommandMismatch {
                        type_name: ::core::any::type_name::<Self>(),
                    });
                }
                Ok(())
            }
        }
    })
}

/// Parsed root schema metadata.
#[derive(Default)]
struct RootSchema {
    /// Optional executable root handler.
    handler: Option<Path>,
    /// Optional application-defined metadata schema type.
    metadata: Option<Type>,
}

/// Parses root `#[schema(...)]` metadata.
fn parse_root_schema(attrs: &[Attribute]) -> syn::Result<RootSchema> {
    let mut result = RootSchema::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("schema")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("handler") {
                if result.handler.is_some() {
                    return Err(meta.error("duplicate root handler"));
                }
                result.handler = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("metadata") {
                if result.metadata.is_some() {
                    return Err(meta.error("duplicate root metadata type"));
                }
                result.metadata = Some(meta.value()?.parse()?);
            } else {
                return Err(meta.error("unsupported #[schema(...)] root option"));
            }
            Ok(())
        })?;
    }
    Ok(result)
}

/// Finds the root `#[command(subcommand)]` field, when present.
fn find_subcommand_field(input: &DeriveInput) -> syn::Result<Option<Type>> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            "CliSchema can only be derived for structs",
        ));
    };
    let mut found = None;
    for field in &data.fields {
        if command_has_flag(&field.attrs, "subcommand")? {
            if found.is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    "CliSchema supports at most one #[command(subcommand)] field",
                ));
            }
            found = Some(unwrap_option(&field.ty));
        }
    }
    Ok(found)
}

/// How a Clap enum variant participates in the subcommand tree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum CommandNesting {
    /// A regular executable command.
    #[default]
    Leaf,
    /// A nested subcommand group.
    Subcommand,
    /// A flattened subcommand enum.
    Flatten,
}

/// Whether a Clap variant contributes to the contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum CommandDisposition {
    /// Normal contract-visible command.
    #[default]
    Normal,
    /// Clap-skipped command.
    Skip,
    /// External subcommand capture.
    External,
}

/// Parsed Clap behavior relevant to contract registration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CommandBehavior {
    /// Position in the subcommand tree.
    nesting: CommandNesting,
    /// Whether the variant is represented.
    disposition: CommandDisposition,
}

/// Parses Clap attributes that affect command-tree registration.
fn parse_command_behavior(attrs: &[Attribute]) -> syn::Result<CommandBehavior> {
    let mut subcommand = false;
    let mut flatten = false;
    let mut skip = false;
    let mut external = false;

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("command")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };
        let metas =
            list.parse_args_with(syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in metas {
            if let Meta::Path(path) = meta {
                if path.is_ident("subcommand") {
                    subcommand = true;
                } else if path.is_ident("flatten") {
                    flatten = true;
                } else if path.is_ident("skip") {
                    skip = true;
                } else if path.is_ident("external_subcommand") {
                    external = true;
                }
            }
        }
    }

    let nesting = match (subcommand, flatten) {
        (false, false) => CommandNesting::Leaf,
        (true, false) => CommandNesting::Subcommand,
        (false, true) => CommandNesting::Flatten,
        (true, true) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "a command cannot be both subcommand and flatten",
            ));
        }
    };
    let disposition = match (skip, external) {
        (false, false) => CommandDisposition::Normal,
        (true, false) => CommandDisposition::Skip,
        (false, true) => CommandDisposition::External,
        (true, true) => {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                "a command cannot be both skip and external_subcommand",
            ));
        }
    };

    Ok(CommandBehavior { nesting, disposition })
}

/// Parsed contract metadata for one subcommand variant.
#[derive(Default)]
struct VariantSchema {
    /// Canonical handler path, when this command is executable.
    handler: Option<Path>,
    /// Explicit child enum when an `Args` payload owns subcommands.
    subcommands: Option<Type>,
    /// Optional operation-specific application metadata schema type.
    metadata: Option<Type>,
    /// Whether this runtime command is omitted from the contract.
    skip: bool,
}

impl VariantSchema {
    /// Returns whether no schema metadata was supplied.
    const fn is_empty(&self) -> bool {
        self.handler.is_none()
            && self.subcommands.is_none()
            && self.metadata.is_none()
            && !self.skip
    }

    /// Returns whether metadata affects operation registration.
    const fn has_operation_metadata(&self) -> bool {
        self.handler.is_some() || self.subcommands.is_some() || self.metadata.is_some()
    }
}

/// Parses operation metadata attached to one subcommand variant.
fn parse_variant_schema(attrs: &[Attribute]) -> syn::Result<VariantSchema> {
    let mut result = VariantSchema::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("schema")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("handler") {
                if result.handler.is_some() {
                    return Err(meta.error("duplicate handler"));
                }
                result.handler = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("subcommands") {
                if result.subcommands.is_some() {
                    return Err(meta.error("duplicate subcommands type"));
                }
                result.subcommands = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("metadata") {
                if result.metadata.is_some() {
                    return Err(meta.error("duplicate metadata type"));
                }
                result.metadata = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("skip") {
                result.skip = true;
            } else {
                return Err(meta.error("unsupported #[schema(...)] command option"));
            }
            Ok(())
        })?;
    }
    Ok(result)
}

/// Returns whether a Clap `#[command(...)]` attribute contains a flag.
fn command_has_flag(attrs: &[Attribute], flag: &str) -> syn::Result<bool> {
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("command")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };
        let metas =
            list.parse_args_with(syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated)?;
        for meta in metas {
            if matches!(&meta, Meta::Path(path) if path.is_ident(flag)) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// Returns the payload type of a one-field tuple variant.
fn single_payload_type(fields: &Fields) -> Option<Type> {
    match fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            fields.unnamed.first().map(|field| field.ty.clone())
        }
        _ => None,
    }
}

/// Unwraps an `Option<T>` root subcommand field.
fn unwrap_option(ty: &Type) -> Type {
    let Type::Path(path) = ty else {
        return ty.clone();
    };
    let Some(segment) = path.path.segments.last() else {
        return ty.clone();
    };
    if segment.ident != "Option" {
        return ty.clone();
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return ty.clone();
    };
    arguments
        .args
        .iter()
        .find_map(|argument| match argument {
            GenericArgument::Type(inner) => Some(inner.clone()),
            _ => None,
        })
        .unwrap_or_else(|| ty.clone())
}

/// Resolves the public `clap_schema` path for generated code.
fn clap_schema_path() -> TokenStream2 {
    match crate_name("clap_schema") {
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Ok(FoundCrate::Itself) | Err(_) => quote!(::clap_schema),
    }
}

#[cfg(test)]
mod tests;
