//! Proc macros for `clap_schema`.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/selemis-com/clap_schema/master/.github/assets/logo.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/selemis-com/clap_schema/master/.github/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Fields, GenericArgument, Ident, ImplItem, ImplItemFn, ItemFn,
    ItemImpl, Meta, PathArguments, ReturnType, Token, Type, parse_macro_input,
};

/// Marks a type as a `clap_schema::Operation`.
///
/// The derive establishes the operation identity locally. Exactly one canonical
/// `#[clap_schema::handler]` must supply the corresponding hidden handler contract;
/// `clap_schema` provides the public `Operation` capability when both are present.
#[proc_macro_derive(Operation)]
pub fn derive_operation(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let crate_path = clap_schema_path();
    let ident = input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    quote! {
        impl #impl_generics #crate_path::__private::OperationMarker for #ident #ty_generics #where_clause {}
    }
    .into()
}

/// Derives the root `clap_schema::CliSchema` implementation.
///
/// The derive reflects the root Clap parser and registers output contracts for
/// its root operation and/or subcommand tree. Invocation syntax stays owned by
/// Clap. An executable root uses the root parser type as its operation type; a
/// `#[clap_schema::handler]` that accepts that type provides its handler contract.
///
/// # `#[schema(...)]` options
///
/// - `executable` makes the root parser type an executable operation; the root must implement
///   `clap_schema::Operation`.
/// - `extend = Type` declares the application-wide extension schema type. It is schema-only:
///   `clap_schema` never constructs or serializes values of `Type`.
///
/// Root `extend` describes the application-wide extension vocabulary, not a supplement specific
/// to an executable root handler. Builder-style applications can supplement individual operation
/// types with `ContractBuilder::operation_with_extension`.
#[proc_macro_derive(CliSchema, attributes(schema, command))]
pub fn derive_cli_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_cli_schema(input).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Derives the child-subcommand type from an `Args` wrapper.
///
/// The input must be a struct with exactly one `#[command(subcommand)]` field. This keeps an
/// executable parent command's child type anchored to the same field Clap parses instead of
/// repeating that type in `#[schema(...)]`.
#[proc_macro_derive(CommandGroup, attributes(command))]
pub fn derive_command_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_command_group(input).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Derives operation registration for a Clap subcommand enum.
///
/// Every contract-visible executable variant has one tuple payload implementing
/// `clap_schema::Operation`. The operation type supplies its successful-output contract through
/// normal Rust trait resolution.
///
/// Normal `#[command(subcommand)]` and `#[command(flatten)]` nesting is followed automatically.
/// When an `Args` payload itself contains a subcommand field, derive `CommandGroup` on that payload
/// and add the `subcommands` flag to the parent variant. Add `executable` when such a parent is
/// also executable without selecting a child. Executable operations may declare `extend = Type` to
/// supplement the root application extension schema. Extensions can only be attached to ordinary
/// executable variants; command groups, flattened variants, skipped variants, and external
/// subcommands do not carry an operation-specific extension schema. The application owns all
/// concrete extension values.
#[proc_macro_derive(CommandSchema, attributes(schema, command))]
pub fn derive_command_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_command_schema(input).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Marks the canonical implementation of an operation type.
///
/// For free functions, the handler must have exactly one typed operation input. The macro
/// implements the hidden handler contract for that input type; borrowing the input does not change
/// its identity.
///
/// Receiver-based handlers use a dedicated inherent impl block:
///
/// ```ignore
/// use clap_schema::Operation;
///
/// #[derive(Operation)]
/// struct CreateCommand;
///
/// #[clap_schema::handler]
/// impl CreateCommand {
///     async fn run(self, context: Context) -> Result<Created, Error> {
///         // ...
///     }
/// }
/// ```
///
/// The handler impl must contain exactly one receiver method. Put unrelated helper methods in a
/// separate inherent impl block. Additional runtime context parameters are unrestricted because the
/// enclosing `Self` type, rather than a parameter position, is the operation identity.
///
/// The handler's declared `Result<T, E>` is the sole source of the successful output contract; type
/// aliases are supported. Generic handlers and opaque `impl Trait` return types are rejected
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
    if let Ok(item_impl) = syn::parse2::<ItemImpl>(tokens.clone()) {
        return expand_handler_impl(&item_impl)
            .unwrap_or_else(syn::Error::into_compile_error)
            .into();
    }
    if let Ok(method) = syn::parse2::<ImplItemFn>(tokens.clone())
        && matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_)))
    {
        return syn::Error::new_spanned(
            method.sig,
            "receiver handlers must put #[clap_schema::handler] on a dedicated inherent impl block",
        )
        .into_compile_error()
        .into();
    }

    let function = match syn::parse2::<ItemFn>(tokens) {
        Ok(function) => function,
        Err(error) => return error.into_compile_error().into(),
    };
    expand_item_handler(&function).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Expands a free handler and its operation-type contract implementation.
fn expand_item_handler(function: &ItemFn) -> syn::Result<TokenStream2> {
    validate_handler_signature(&function.sig)?;
    let output = declared_return_type(&function.sig)?;
    let conditional = conditional_attributes(&function.attrs)?;
    let crate_path = clap_schema_path();
    let operation = free_handler_operation_type(&function.sig)?;

    Ok(quote! {
        #function

        #(#conditional)*
        impl #crate_path::__private::HandlerContract for #operation {
            type Output = <#output as #crate_path::__private::HandlerResult>::Output;
        }
    })
}

/// Expands a dedicated inherent handler impl and its operation-type contract implementation.
fn expand_handler_impl(item_impl: &ItemImpl) -> syn::Result<TokenStream2> {
    if item_impl.trait_.is_some() {
        return Err(syn::Error::new_spanned(
            item_impl,
            "#[clap_schema::handler] requires an inherent impl block",
        ));
    }
    if !item_impl.generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            &item_impl.generics,
            "clap_schema handler impls require a concrete non-generic operation type",
        ));
    }

    let mut methods = item_impl.items.iter().filter_map(|item| match item {
        ImplItem::Fn(method) => Some(method),
        _ => None,
    });
    let Some(method) = methods.next() else {
        return Err(syn::Error::new_spanned(
            item_impl,
            "#[clap_schema::handler] impl blocks must contain exactly one function; put helpers in a separate impl block",
        ));
    };
    if methods.next().is_some() {
        return Err(syn::Error::new_spanned(
            item_impl,
            "#[clap_schema::handler] impl blocks must contain exactly one function; put helpers in a separate impl block",
        ));
    }

    if !matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_))) {
        return Err(syn::Error::new_spanned(
            &method.sig,
            "#[clap_schema::handler] impl blocks require a receiver method so Self is the operation identity",
        ));
    }

    validate_handler_signature(&method.sig)?;
    let output = declared_return_type(&method.sig)?;
    let mut conditional = conditional_attributes(&item_impl.attrs)?;
    conditional.extend(conditional_attributes(&method.attrs)?);
    let crate_path = clap_schema_path();
    let operation = &item_impl.self_ty;
    let generics = &item_impl.generics;
    let (impl_generics, _, where_clause) = generics.split_for_impl();

    Ok(quote! {
        #item_impl

        #(#conditional)*
        impl #impl_generics #crate_path::__private::HandlerContract for #operation #where_clause {
            type Output = <#output as #crate_path::__private::HandlerResult>::Output;
        }
    })
}

/// Returns conditional-compilation attributes that must also guard the generated companion.
fn conditional_attributes(attrs: &[Attribute]) -> syn::Result<Vec<TokenStream2>> {
    let mut conditional = Vec::new();
    for attribute in attrs {
        if attribute.path().is_ident("cfg") {
            conditional.push(quote!(#attribute));
            continue;
        }
        if !attribute.path().is_ident("cfg_attr") {
            continue;
        }

        let Meta::List(list) = &attribute.meta else {
            continue;
        };
        let nested =
            list.parse_args_with(syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated)?;
        let mut nested = nested.into_iter();
        let Some(predicate) = nested.next() else {
            continue;
        };
        let cfg_attributes = nested.filter(|meta| meta.path().is_ident("cfg")).collect::<Vec<_>>();
        if !cfg_attributes.is_empty() {
            conditional.push(quote!(#[cfg_attr(#predicate, #(#cfg_attributes),*)]));
        }
    }
    Ok(conditional)
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

/// Resolves the Rust operation type of a free handler.
fn free_handler_operation_type(signature: &syn::Signature) -> syn::Result<Type> {
    let inputs = signature
        .inputs
        .iter()
        .filter_map(|input| match input {
            syn::FnArg::Typed(input) => Some(input.ty.as_ref()),
            syn::FnArg::Receiver(_) => None,
        })
        .collect::<Vec<_>>();
    let [input] = inputs.as_slice() else {
        return Err(syn::Error::new_spanned(
            &signature.inputs,
            "free clap_schema handlers require exactly one typed operation input; use a dedicated #[clap_schema::handler] impl block when runtime context parameters are needed",
        ));
    };
    let input = command_input_type(input);
    if is_self_type(&input) {
        return Err(syn::Error::new_spanned(
            input,
            "associated handlers must put #[clap_schema::handler] on a dedicated inherent impl block",
        ));
    }
    if !matches!(input, Type::Path(_)) {
        return Err(syn::Error::new_spanned(
            input,
            "clap_schema operation inputs must be named types",
        ));
    }
    Ok(input)
}

/// Removes borrowing/grouping syntax from a handler command input.
fn command_input_type(ty: &Type) -> Type {
    match ty {
        Type::Reference(reference) => command_input_type(&reference.elem),
        Type::Group(group) => command_input_type(&group.elem),
        Type::Paren(paren) => command_input_type(&paren.elem),
        _ => ty.clone(),
    }
}

/// Returns whether a type is the enclosing inherent implementation's `Self` type.
fn is_self_type(ty: &Type) -> bool {
    let Type::Path(path) = ty else {
        return false;
    };
    path.qself.is_none()
        && path.path.leading_colon.is_none()
        && path.path.segments.len() == 1
        && path.path.segments.first().is_some_and(|segment| segment.ident == "Self")
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

/// Expands a `CliSchema` derive into root operation registration.
fn expand_cli_schema(input: DeriveInput) -> syn::Result<TokenStream2> {
    let crate_path = clap_schema_path();
    let RootSchema { executable, extended } = parse_root_schema(&input.attrs)?;
    let commands = find_subcommand_field(&input, "CliSchema")?;

    let name = input.ident;
    let generics = input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let root_extended = extended.map(|extended| {
        quote! {
            registry.extend::<#extended>();
        }
    });
    let root_operation = executable.then(|| {
        quote! {
            registry.operation::<Self>(&[]);
        }
    });
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
                #root_extended
                #root_operation
                #child_registration
                Ok(())
            }
        }
    })
}

/// Expands a `CommandGroup` derive into its child-subcommand association.
fn expand_command_group(input: DeriveInput) -> syn::Result<TokenStream2> {
    let crate_path = clap_schema_path();
    let commands = find_subcommand_field(&input, "CommandGroup")?.ok_or_else(|| {
        syn::Error::new_spanned(
            &input.ident,
            "CommandGroup requires one #[command(subcommand)] field",
        )
    })?;

    let name = input.ident;
    let generics = input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics #crate_path::CommandGroup for #name #type_generics #where_clause {
            type Subcommands = #commands;
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
                    "schema extensions cannot be attached to a clap-skipped or external subcommand variant",
                ));
            }
            continue;
        }

        if schema.skip && schema.has_registration_options() {
            return Err(syn::Error::new_spanned(
                variant.ident,
                "#[schema(skip)] cannot be combined with executable, subcommands, or extend",
            ));
        }

        if command.nesting == CommandNesting::Flatten {
            let child = payload.ok_or_else(|| {
                syn::Error::new_spanned(
                    &variant.ident,
                    "flattened subcommands require a single tuple payload",
                )
            })?;
            if schema.has_registration_options() {
                return Err(syn::Error::new_spanned(
                    variant.ident,
                    "flattened subcommands cannot declare operation schema extensions",
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
            if schema.has_registration_options() {
                return Err(syn::Error::new_spanned(
                    variant.ident,
                    "#[command(subcommand)] groups cannot declare executable, subcommands, or extend",
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

        let payload = payload.ok_or_else(|| {
            syn::Error::new_spanned(
                &variant.ident,
                "contract-visible executable commands require a single tuple Args payload implementing clap_schema::Operation",
            )
        })?;
        if schema.executable && !schema.subcommands {
            return Err(syn::Error::new_spanned(
                &variant.ident,
                "the `executable` flag is only needed when `subcommands` is also declared",
            ));
        }
        if schema.extended.is_some() && schema.subcommands && !schema.executable {
            return Err(syn::Error::new_spanned(
                &variant.ident,
                "an extension on a command group requires the `executable` flag",
            ));
        }

        let register_operation = (!schema.subcommands || schema.executable).then(|| {
            schema.extended.as_ref().map_or_else(
                || {
                    quote! {
                        registry.operation::<#payload>(prefix);
                    }
                },
                |extended| {
                    quote! {
                        registry.operation_extended::<#payload, #extended>(prefix);
                    }
                },
            )
        });
        let register_children = schema.subcommands.then(|| quote! {
                type __ClapSchemaChildren =
                    <#payload as #crate_path::CommandGroup>::Subcommands;
                let __clap_schema_child_probe =
                    <__ClapSchemaChildren as #crate_path::__private::clap::Subcommand>::augment_subcommands(
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
                                type_name: ::core::any::type_name::<__ClapSchemaChildren>(),
                            });
                        }
                    }
                }
                <__ClapSchemaChildren as #crate_path::CommandSchema>::__clap_schema_register(
                    prefix,
                    registry,
                )?;
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
                #register_operation
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

/// Parsed root schema extensions.
#[derive(Default)]
struct RootSchema {
    /// Whether the root parser is executable.
    executable: bool,
    /// Optional application-defined extension schema type.
    extended: Option<Type>,
}

/// Parses root `#[schema(...)]` extensions.
fn parse_root_schema(attrs: &[Attribute]) -> syn::Result<RootSchema> {
    let mut result = RootSchema::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("schema")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("executable") {
                if result.executable {
                    return Err(meta.error("duplicate executable flag"));
                }
                if meta.input.peek(Token![=]) {
                    return Err(meta.error("`executable` is a flag and does not accept a value"));
                }
                result.executable = true;
            } else if meta.path.is_ident("extend") {
                if result.extended.is_some() {
                    return Err(meta.error("duplicate root extension type"));
                }
                result.extended = Some(meta.value()?.parse()?);
            } else {
                return Err(meta.error("unsupported #[schema(...)] root option"));
            }
            Ok(())
        })?;
    }
    Ok(result)
}

/// Finds one `#[command(subcommand)]` field, when present.
fn find_subcommand_field(input: &DeriveInput, derive_name: &str) -> syn::Result<Option<Type>> {
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input.ident,
            format!("{derive_name} can only be derived for structs"),
        ));
    };
    let mut found = None;
    for field in &data.fields {
        if command_has_flag(&field.attrs, "subcommand")? {
            if found.is_some() {
                return Err(syn::Error::new_spanned(
                    field,
                    format!("{derive_name} supports at most one #[command(subcommand)] field"),
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

/// Parsed contract extensions for one subcommand variant.
#[derive(Default)]
struct VariantSchema {
    /// Whether a command group is executable without selecting a child.
    executable: bool,
    /// Whether an `Args` payload owns child subcommands through `CommandGroup`.
    subcommands: bool,
    /// Optional operation-specific application extension schema type.
    extended: Option<Type>,
    /// Whether this runtime command is omitted from the contract.
    skip: bool,
}

impl VariantSchema {
    /// Returns whether no schema extension was supplied.
    const fn is_empty(&self) -> bool {
        !self.executable && !self.subcommands && self.extended.is_none() && !self.skip
    }

    /// Returns whether extensions affect operation or child registration.
    const fn has_registration_options(&self) -> bool {
        self.executable || self.subcommands || self.extended.is_some()
    }
}

/// Parses operation extensions attached to one subcommand variant.
fn parse_variant_schema(attrs: &[Attribute]) -> syn::Result<VariantSchema> {
    let mut result = VariantSchema::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("schema")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("executable") {
                if result.executable {
                    return Err(meta.error("duplicate executable flag"));
                }
                if meta.input.peek(Token![=]) {
                    return Err(meta.error("`executable` is a flag and does not accept a value"));
                }
                result.executable = true;
            } else if meta.path.is_ident("subcommands") {
                if result.subcommands {
                    return Err(meta.error("duplicate subcommands flag"));
                }
                if meta.input.peek(Token![=]) {
                    return Err(meta.error("`subcommands` is a flag and does not accept a value"));
                }
                result.subcommands = true;
            } else if meta.path.is_ident("extend") {
                if result.extended.is_some() {
                    return Err(meta.error("duplicate extension type"));
                }
                result.extended = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("skip") {
                if result.skip {
                    return Err(meta.error("duplicate skip flag"));
                }
                if meta.input.peek(Token![=]) {
                    return Err(meta.error("`skip` is a flag and does not accept a value"));
                }
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
