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
/// Clap. Executable roots are wired through the root parser type itself: a
/// `#[clap_schema::handler]` that accepts that type provides its operation metadata.
///
/// # `#[schema(...)]` options
///
/// - `executable` binds the root parser type as an executable operation.
/// - `extend = Type` declares the application-wide extension schema type. It is schema-only:
///   `clap_schema` never constructs or serializes values of `Type`.
///
/// Root `extend` describes the application-wide extension vocabulary, not a supplement specific
/// to an executable root handler. Builder-style applications can supplement a registered root
/// operation directly through `Operation::extend`.
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

/// Derives type-driven operation registration for a Clap subcommand enum.
///
/// Every contract-visible executable variant has one tuple payload whose type is also accepted by
/// exactly one `#[clap_schema::handler]`. The handler macro installs hidden operation metadata on
/// that payload type, so there is no handler path to repeat on the enum variant. Rust name and type
/// checking therefore owns the association.
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

/// Marks the canonical implementation of a contract-visible operation.
///
/// The attribute leaves runtime behavior unchanged and generates hidden operation metadata. The
/// declared `Result<T, E>` return type is the sole source of the successful output contract; type
/// aliases are supported.
///
/// A handler with one typed command argument binds that argument's named type to the operation. An
/// inherent method with a receiver binds `Self`. `CommandSchema` uses that type-level association
/// directly, so derive-based command definitions never repeat a handler path. Zero-argument
/// handlers remain available to builder-style applications through [`operation!`].
///
/// Sync, `const fn`, async, free functions, associated functions whose command input is `Self`,
/// and inherent methods with any receiver form are supported. Generic handlers and handlers with
/// more than one typed argument are rejected because they do not identify one concrete command
/// input and output contract.
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

/// Returns handler-derived operation metadata and identity.
///
/// Builder-style Clap uses the returned `clap_schema::Operation` when registering a command path.
/// Derive-based code can pass it to `CliContract::command_for` or
/// `CliContract::extended_schema_for_operation` to query the command already wired through the
/// handler's input type. It has no syntax for declaring an output type manually.
/// Application-defined schema extensions can be added with `Operation::extend`.
#[proc_macro]
pub fn operation(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as Path);
    match handler_helper_path(path) {
        Ok(helper) => quote!(#helper()).into(),
        Err(error) => error.into_compile_error().into(),
    }
}

/// How a handler is bound to a command input type for derive registration.
enum HandlerBinding {
    /// Builder-only handler with no command input type.
    None,
    /// Inherent handler whose receiver or typed argument is `Self`.
    SelfType,
    /// Free or associated handler bound to one named command input type.
    Type(Box<Type>),
}

/// Expands a free or associated handler and its metadata companion.
fn expand_item_handler(function: &ItemFn) -> syn::Result<TokenStream2> {
    validate_handler_signature(&function.sig)?;
    let helper = handler_helper_ident(&function.sig.ident);
    let binding_helper = operation_binding_ident();
    let output = declared_return_type(&function.sig)?;
    let visibility = &function.vis;
    let conditional = conditional_attributes(&function.attrs)?;
    let crate_path = clap_schema_path();
    let binding = handler_binding(&function.sig)?;

    let helper_body = match &binding {
        HandlerBinding::None => quote! {
            #[doc = "Identity marker for this annotated handler."]
            struct __ClapSchemaHandlerIdentity;
            #crate_path::__private::operation_from_result::<
                #output,
                __ClapSchemaHandlerIdentity,
            >()
        },
        HandlerBinding::SelfType => quote! {
            #crate_path::__private::operation_from_result::<#output, Self>()
        },
        HandlerBinding::Type(command) => quote! {
            #crate_path::__private::operation_from_result::<#output, #command>()
        },
    };

    let binding = match binding {
        HandlerBinding::None => None,
        HandlerBinding::SelfType => Some(quote! {
            #(#conditional)*
            #[doc(hidden)]
            #[doc = "Generated clap_schema type-driven operation binding."]
            pub fn #binding_helper() -> #crate_path::Operation {
                #crate_path::__private::operation_from_result::<#output, Self>()
            }
        }),
        HandlerBinding::Type(command) => Some(quote! {
            #(#conditional)*
            impl #command {
                #[doc(hidden)]
                #[doc = "Generated clap_schema type-driven operation binding."]
                pub fn #binding_helper() -> #crate_path::Operation {
                    #crate_path::__private::operation_from_result::<#output, #command>()
                }
            }
        }),
    };

    Ok(quote! {
        #function

        #(#conditional)*
        #[doc(hidden)]
        #[doc = "Generated clap_schema operation metadata."]
        #visibility fn #helper() -> #crate_path::Operation {
            #helper_body
        }

        #binding
    })
}

/// Expands an inherent receiver method and its metadata companion.
fn expand_impl_handler(method: &ImplItemFn) -> syn::Result<TokenStream2> {
    validate_handler_signature(&method.sig)?;
    if method.sig.inputs.iter().filter(|input| matches!(input, syn::FnArg::Typed(_))).count() != 0 {
        return Err(syn::Error::new_spanned(
            &method.sig.inputs,
            "receiver handlers cannot declare an additional command input",
        ));
    }

    let helper = handler_helper_ident(&method.sig.ident);
    let binding_helper = operation_binding_ident();
    let output = declared_return_type(&method.sig)?;
    let visibility = &method.vis;
    let conditional = conditional_attributes(&method.attrs)?;
    let crate_path = clap_schema_path();

    Ok(quote! {
        #method

        #(#conditional)*
        #[doc(hidden)]
        #[doc = "Generated clap_schema operation metadata."]
        #visibility fn #helper() -> #crate_path::Operation {
            #crate_path::__private::operation_from_result::<#output, Self>()
        }

        #(#conditional)*
        #[doc(hidden)]
        #[doc = "Generated clap_schema type-driven operation binding."]
        pub fn #binding_helper() -> #crate_path::Operation {
            #crate_path::__private::operation_from_result::<#output, Self>()
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

/// Resolves the command input type that a handler binds for derive registration.
fn handler_binding(signature: &syn::Signature) -> syn::Result<HandlerBinding> {
    if signature.inputs.iter().any(|input| matches!(input, syn::FnArg::Receiver(_))) {
        return Ok(HandlerBinding::SelfType);
    }

    let inputs = signature
        .inputs
        .iter()
        .filter_map(|input| match input {
            syn::FnArg::Typed(input) => Some(input.ty.as_ref()),
            syn::FnArg::Receiver(_) => None,
        })
        .collect::<Vec<_>>();
    match inputs.as_slice() {
        [] => Ok(HandlerBinding::None),
        [input] => {
            let input = command_input_type(input);
            if is_self_type(&input) {
                Ok(HandlerBinding::SelfType)
            } else if matches!(input, Type::Path(_)) {
                Ok(HandlerBinding::Type(Box::new(input)))
            } else {
                Err(syn::Error::new_spanned(
                    input,
                    "clap_schema command inputs must be named types",
                ))
            }
        }
        _ => Err(syn::Error::new_spanned(
            &signature.inputs,
            "clap_schema handlers accept at most one typed command input",
        )),
    }
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

/// Returns the payload-level operation binding helper shared by all derive registration.
fn operation_binding_ident() -> Ident {
    format_ident!("__clap_schema_operation")
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
        let binding = operation_binding_ident();
        quote! {
            registry.operation(&[], Self::#binding());
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
    let binding = operation_binding_ident();
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
                "contract-visible executable commands require a single tuple Args payload accepted by #[clap_schema::handler]",
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
            let extended = schema.extended.as_ref().map(|extended| {
                quote! { .extend::<#extended>() }
            });
            quote! {
                registry.operation(prefix, <#payload>::#binding() #extended);
            }
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
            } else if meta.path.is_ident("handler") {
                return Err(meta.error(
                    "handler paths are no longer declared in #[schema(...)]; remove `handler = ...`, add `executable` when the root itself executes, and let #[clap_schema::handler] bind the root type",
                ));
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
            } else if meta.path.is_ident("handler") {
                return Err(meta.error(
                    "handler paths are no longer declared in #[schema(...)]; remove `handler = ...` and let #[clap_schema::handler] bind the variant payload type directly",
                ));
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
