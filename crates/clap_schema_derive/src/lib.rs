//! Proc macros for `clap_schema`.

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, FnArg, GenericArgument, Ident, ImplItemFn, ItemFn,
    LitStr, Meta, PathArguments, ReturnType, Token, Type, parse_macro_input,
};

/// Derives the root `clap_schema::CliSchema` implementation.
///
/// The derive reflects the root Clap parser and locates its
/// `#[command(subcommand)]` field. Root arguments become invocation context;
/// executable operations come from the associated `CommandSchema` enum.
///
/// # `#[schema(...)]` options
///
/// - `#[schema(include_hidden)]` includes Clap-hidden commands in the contract.
/// - `#[schema(json_output = "json")]` names a root argument that enables JSON output. The argument
///   may be a boolean flag or a value-taking option.
/// - `#[schema(json_output = "format", json_value = "json")]` selects JSON by assigning a
///   particular value to the named option. `json_value` is invalid without `json_output`.
///
/// When Clap exposes a finite value set for a value-taking output selector,
/// contract construction validates the configured `json_value` against it.
/// Without an explicit `json_output`, the runtime defaults to the library's
/// `JsonOutput::Auto` policy.
///
/// Root-only executable operations are outside the 0.1 contract model: root
/// arguments are reserved for invocation context and contract-visible
/// operations are subcommand leaves.
#[proc_macro_derive(CliSchema, attributes(schema, command))]
pub fn derive_cli_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_cli_schema(input).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Derives typed leaf command registration for a Clap subcommand enum.
///
/// `CommandSchema` recursively follows normal Clap nesting and flattening while
/// preserving canonical command names, paths, help text, and argument syntax.
/// A contract-visible executable leaf must be a one-field tuple variant such as
/// `Create(CreateArgs)`. Its payload type is the key that joins the Clap leaf to
/// the canonical `#[clap_schema::handler]` for that operation.
///
/// Intermediate command groups and variants marked `#[schema(skip)]` do not need
/// handlers. Flattened subcommand enums use Clap's `#[command(flatten)]` and do
/// not add a path component.
///
/// # `#[schema(...)]` options
///
/// The following options are valid on executable leaf variants:
///
/// - `skip` — omit a runtime-only variant from the contract.
/// - `input = Request` — use `Request: JsonSchema` as semantic input instead of the Clap payload
///   type.
/// - `deprecated = "guidance"` — attach deprecation or migration guidance.
/// - `structured = "input"` — treat the named Clap argument as a complete JSON source transport.
/// - `stdin = "-"` — declare the exact structured-source token representing standard input.
///   Requires `structured`.
/// - `structured_only` — suppress ordinary property-by-property argv transport. Requires
///   `structured`.
/// - `json(metadata, filters)` — serialize the named semantic properties as complete JSON argv
///   tokens.
/// - `bind(query = "q")` — bind a semantic property to a differently named Clap argument.
///
/// Leaf-only metadata is invalid on intermediate or flattened command groups.
/// A property named by both `bind(...)` and `json(...)` keeps the explicit Clap
/// binding and uses JSON value encoding.
///
/// By default the leaf payload type supplies the semantic input schema. An
/// `input = Request` override changes only the machine-facing semantic schema;
/// the original payload remains the runtime Clap carrier and handler key.
#[proc_macro_derive(CommandSchema, attributes(schema, command))]
pub fn derive_command_schema(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_command_schema(input).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Marks the canonical handler for a contract-visible command payload.
///
/// Free handlers use their first argument as the command payload type. Inherent
/// methods use their enclosing `Self` type as the payload key. The generated
/// metadata uses the
/// function signature only as a compile-time type witness: the handler is never
/// called while a contract is constructed.
///
/// Rust resolves either `Result<T, E>` or `Future<Output = Result<T, E>>`, and
/// Schemars generates the successful schema for `T`. The error type `E` is
/// deliberately ignored and needs no `JsonSchema` implementation.
/// `Result<(), E>` means the command has no successful payload.
///
/// # Supported forms
///
/// The 0.1 handler model supports synchronous and asynchronous free functions,
/// and synchronous or asynchronous inherent methods with `self`, `&self`, or
/// `&mut self` receivers. Synchronous handlers may also be `const fn`. Rust does
/// not permit `async const fn`, so const and async are separate forms.
///
/// ```text
/// #[clap_schema::handler]
/// fn create(command: CreateArgs, ctx: &Context) -> Result<Item, Error> { ... }
///
/// #[clap_schema::handler]
/// async fn fetch(command: FetchArgs, ctx: &Context) -> Result<Item, Error> { ... }
///
/// impl UpdateArgs {
///     #[clap_schema::handler]
///     fn run(self, ctx: &Context) -> Result<Item, Error> { ... }
/// }
///
/// impl DeleteArgs {
///     #[clap_schema::handler]
///     async fn run(self, ctx: &Context) -> Result<(), Error> { ... }
/// }
///
/// impl InspectArgs {
///     #[clap_schema::handler]
///     fn run(&self, ctx: &Context) -> Result<Item, Error> { ... }
/// }
///
/// impl RefreshArgs {
///     #[clap_schema::handler]
///     async fn run(&mut self, ctx: &Context) -> Result<Item, Error> { ... }
/// }
/// ```
///
/// Handlers are intentionally plain and non-generic. Free handlers must own a
/// named local payload in their first argument. Method handlers may use any
/// ordinary inherent receiver form (`self`, `&self`, or `&mut self`); the
/// enclosing `Self` type remains the command payload key. Borrowed free-function
/// payloads, generic handlers, associated functions without `self`, and
/// trait-object registration are unsupported.
/// Additional arguments are runtime context only and do not participate in the
/// input schema.
///
/// One payload type has one canonical handler. Contract-visible subcommand
/// variants therefore use distinct one-field tuple payloads; reusable Clap
/// argument groups should be flattened inside those payloads instead of sharing
/// one payload across multiple executable leaves.
///
/// # Runtime dispatch
///
/// `clap_schema` does not own execution. Applications continue to dispatch with
/// ordinary matches and call the same functions or methods directly. The
/// attribute only exposes the handler's successful type-level contract to the
/// `CommandSchema` derive.
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
        && matches!(method.sig.inputs.first(), Some(FnArg::Receiver(_)))
    {
        return expand_method_handler(&method)
            .unwrap_or_else(syn::Error::into_compile_error)
            .into();
    }

    let function = match syn::parse2::<ItemFn>(tokens) {
        Ok(function) => function,
        Err(error) => return error.into_compile_error().into(),
    };
    expand_free_handler(&function).unwrap_or_else(syn::Error::into_compile_error).into()
}

/// Expands a free handler function and its payload-keyed metadata.
fn expand_free_handler(function: &ItemFn) -> syn::Result<TokenStream2> {
    let crate_path = clap_schema_path();
    let signature = &function.sig;
    validate_handler_signature(signature)?;

    let first = signature.inputs.first().ok_or_else(|| {
        syn::Error::new_spanned(
            signature,
            "clap_schema handlers require a command payload argument",
        )
    })?;
    let FnArg::Typed(first) = first else {
        return Err(syn::Error::new_spanned(
            first,
            "free clap_schema handlers cannot have a self receiver",
        ));
    };
    if matches!(first.ty.as_ref(), Type::Reference(_)) {
        return Err(syn::Error::new_spanned(
            &first.ty,
            "the first #[clap_schema::handler] argument must own the command payload",
        ));
    }
    if !matches!(first.ty.as_ref(), Type::Path(_)) {
        return Err(syn::Error::new_spanned(
            &first.ty,
            "the first #[clap_schema::handler] argument must be a named command payload type",
        ));
    }
    let payload_ty = first.ty.as_ref();

    let mut argument_types = Vec::new();
    for argument in &signature.inputs {
        let FnArg::Typed(argument) = argument else {
            return Err(syn::Error::new_spanned(
                argument,
                "free clap_schema handlers cannot have a self receiver",
            ));
        };
        argument_types.push(argument.ty.as_ref());
    }
    let name = &signature.ident;
    let command_spec = handler_command_spec(
        &crate_path,
        signature.asyncness.is_some(),
        &quote! { #name(#(#crate_path::__private::type_witness::<#argument_types>()),*) },
    );

    Ok(quote! {
        #function

        impl #payload_ty {
            #[doc(hidden)]
            #[doc = "Builds clap_schema metadata for this command payload."]
            pub(crate) fn __clap_schema_handler_contract<Input>() -> #crate_path::CommandSpec
            where
                Input: ?Sized + #crate_path::JsonSchema,
            {
                #command_spec
            }
        }
    })
}

/// Expands an inherent handler method with a `self` receiver.
fn expand_method_handler(method: &ImplItemFn) -> syn::Result<TokenStream2> {
    let crate_path = clap_schema_path();
    let signature = &method.sig;
    validate_handler_signature(signature)?;

    let first = signature.inputs.first().ok_or_else(|| {
        syn::Error::new_spanned(signature, "clap_schema handler methods require a self receiver")
    })?;
    let FnArg::Receiver(receiver) = first else {
        return Err(syn::Error::new_spanned(
            first,
            "associated clap_schema handlers without self are not supported; use a free function or a method with self, &self, or &mut self",
        ));
    };
    let mut argument_types = Vec::new();
    for argument in signature.inputs.iter().skip(1) {
        let FnArg::Typed(argument) = argument else {
            return Err(syn::Error::new_spanned(
                argument,
                "only the first #[clap_schema::handler] argument may be self",
            ));
        };
        argument_types.push(argument.ty.as_ref());
    }
    let name = &signature.ident;
    let receiver_ty = receiver.ty.as_ref();
    let invocation = quote! {
        Self::#name(
            #crate_path::__private::type_witness::<#receiver_ty>(),
            #(#crate_path::__private::type_witness::<#argument_types>()),*
        )
    };
    let command_spec =
        handler_command_spec(&crate_path, signature.asyncness.is_some(), &invocation);

    Ok(quote! {
        #method

        #[doc(hidden)]
        #[doc = "Builds clap_schema metadata for this command payload."]
        pub(crate) fn __clap_schema_handler_contract<Input>() -> #crate_path::CommandSpec
        where
            Input: ?Sized + #crate_path::JsonSchema,
        {
            #command_spec
        }
    })
}

/// Validates signature properties shared by free and inherent handlers.
fn validate_handler_signature(signature: &syn::Signature) -> syn::Result<()> {
    for argument in &signature.inputs {
        if let FnArg::Typed(argument) = argument
            && matches!(argument.ty.as_ref(), Type::ImplTrait(_))
        {
            return Err(syn::Error::new_spanned(
                &argument.ty,
                "clap_schema handler arguments cannot use impl Trait; use a concrete type or trait object",
            ));
        }
    }

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
    if matches!(&signature.output, ReturnType::Default) {
        return Err(syn::Error::new_spanned(
            signature,
            "clap_schema handlers must return Result<T, E>",
        ));
    }
    Ok(())
}

/// Generates the appropriate sync or async result type witness.
fn handler_command_spec(
    crate_path: &TokenStream2,
    is_async: bool,
    invocation: &TokenStream2,
) -> TokenStream2 {
    if is_async {
        quote! {
            #crate_path::__private::command_spec_from_async::<Input, _, _, _, _>(
                &|| #invocation
            )
        }
    } else {
        quote! {
            #crate_path::__private::command_spec_from_sync::<Input, _, _, _>(
                &|| #invocation
            )
        }
    }
}

/// Expands a `CliSchema` derive into its root schema implementation.
fn expand_cli_schema(input: DeriveInput) -> syn::Result<TokenStream2> {
    let crate_path = clap_schema_path();
    let RootSchema { json_output, json_value, include_hidden } = parse_root_schema(&input.attrs)?;
    let commands = find_subcommand_field(&input)?;
    let name = input.ident;
    let generics = input.generics;
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

    let output = json_output.map(|argument| {
        json_value.map_or_else(
            || {
                quote! {
                    __spec = __spec.json_output(#crate_path::JsonOutput::flag(#argument));
                }
            },
            |value| {
                quote! {
                    __spec = __spec.json_output(#crate_path::JsonOutput::value(#argument, #value));
                }
            },
        )
    });
    let include_hidden = include_hidden.then(|| {
        quote! {
            __spec = __spec.include_hidden();
        }
    });

    Ok(quote! {
        impl #impl_generics #crate_path::CliSchema for #name #type_generics #where_clause {
            type Commands = #commands;

            fn __clap_schema_root() -> #crate_path::__private::RootSpec {
                let mut __spec = #crate_path::__private::RootSpec::default();
                #output
                #include_hidden
                __spec
            }
        }
    })
}

/// Expands a `CommandSchema` derive into leaf and nested command registration.
fn expand_command_schema(input: DeriveInput) -> syn::Result<TokenStream2> {
    let crate_path = clap_schema_path();
    if !input.generics.params.is_empty() || input.generics.where_clause.is_some() {
        return Err(syn::Error::new_spanned(
            &input.generics,
            "CommandSchema currently requires a non-generic subcommand enum",
        ));
    }
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

        if command.nesting == CommandNesting::Flatten {
            let child = payload.ok_or_else(|| {
                syn::Error::new_spanned(
                    &variant.ident,
                    "flattened subcommands require a single tuple payload",
                )
            })?;
            if schema.has_leaf_metadata() {
                return Err(syn::Error::new_spanned(
                    variant.ident,
                    "flattened subcommands cannot declare leaf schema metadata",
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
            if schema.has_leaf_metadata() {
                return Err(syn::Error::new_spanned(
                    variant.ident,
                    "subcommand groups cannot declare leaf schema metadata",
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

        if schema.stdin.is_some() && schema.structured.is_none() {
            return Err(syn::Error::new_spanned(
                &variant.ident,
                "#[schema(stdin = ...)] requires #[schema(structured = ...)]",
            ));
        }
        if schema.structured_only && schema.structured.is_none() {
            return Err(syn::Error::new_spanned(
                &variant.ident,
                "#[schema(structured_only)] requires #[schema(structured = ...)]",
            ));
        }

        let handler_ty = match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                fields.unnamed.first().expect("length checked").ty.clone()
            }
            Fields::Unit | Fields::Named(_) | Fields::Unnamed(_) => {
                return Err(syn::Error::new_spanned(
                    variant.ident,
                    "contract-visible leaf commands require a one-field tuple payload so #[clap_schema::handler] can attach the handler contract",
                ));
            }
        };
        let input_ty = schema.input.unwrap_or_else(|| handler_ty.clone());

        let mut modifiers = Vec::new();
        if let Some(deprecated) = schema.deprecated {
            modifiers.push(quote! { __spec = __spec.deprecated(#deprecated); });
        }
        for (property, argument) in &schema.bindings {
            if schema.json_properties.iter().any(|candidate| candidate.value() == property.value())
            {
                modifiers.push(quote! { __spec = __spec.bind_json(#property, #argument); });
            } else {
                modifiers.push(quote! { __spec = __spec.bind(#property, #argument); });
            }
        }
        for property in &schema.json_properties {
            if !schema.bindings.iter().any(|(candidate, _)| candidate.value() == property.value()) {
                modifiers.push(quote! { __spec = __spec.json(#property); });
            }
        }
        if let Some(argument) = &schema.structured {
            let structured = schema.stdin.as_ref().map_or_else(
                || quote! { #crate_path::StructuredInput::json(#argument) },
                |stdin| quote! { #crate_path::StructuredInput::json(#argument).stdin(#stdin) },
            );
            modifiers.push(quote! { __spec = __spec.structured_input(#structured); });
        }
        if schema.structured_only {
            modifiers.push(quote! { __spec = __spec.structured_only(); });
        }

        steps.push(quote! {
            {
                #consume
                prefix.push(__clap_schema_command.get_name().to_owned());
                let mut __spec =
                    <#handler_ty>::__clap_schema_handler_contract::<#input_ty>();
                #(#modifiers)*
                registry.command(prefix, __spec);
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

/// Parsed `#[schema(...)]` metadata attached to a root CLI type.
#[derive(Default)]
struct RootSchema {
    /// Argument that requests JSON output.
    json_output: Option<LitStr>,
    /// Optional argument value that selects JSON output.
    json_value: Option<LitStr>,
    /// Whether hidden commands should be included in the generated contract.
    include_hidden: bool,
}

/// Parses root-level `#[schema(...)]` metadata.
fn parse_root_schema(attrs: &[Attribute]) -> syn::Result<RootSchema> {
    let mut result = RootSchema::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("schema")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("json_output") {
                result.json_output = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("json_value") {
                result.json_value = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("include_hidden") {
                result.include_hidden = true;
            } else {
                return Err(meta.error("unsupported #[schema(...)] root option"));
            }
            Ok(())
        })?;
    }
    if result.json_output.is_none()
        && let Some(json_value) = &result.json_value
    {
        return Err(syn::Error::new_spanned(json_value, "json_value requires json_output"));
    }
    Ok(result)
}

/// Finds and returns the type of the root `#[command(subcommand)]` field.
fn find_subcommand_field(input: &DeriveInput) -> syn::Result<Type> {
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
                    "CliSchema requires exactly one #[command(subcommand)] field",
                ));
            }
            found = Some(unwrap_option(&field.ty));
        }
    }
    found.ok_or_else(|| {
        syn::Error::new_spanned(&input.ident, "CliSchema requires a #[command(subcommand)] field")
    })
}

/// How a clap enum variant participates in the subcommand tree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum CommandNesting {
    /// A regular executable leaf command.
    #[default]
    Leaf,
    /// A command that owns another subcommand enum.
    Subcommand,
    /// A subcommand enum flattened into its parent command.
    Flatten,
}

/// Whether a clap enum variant participates in the generated contract.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum CommandDisposition {
    /// Include the variant in the generated contract.
    #[default]
    Normal,
    /// Ignore a clap-skipped variant.
    Skip,
    /// Ignore an external subcommand variant.
    External,
}

/// Parsed clap behavior relevant to command-contract registration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CommandBehavior {
    /// The variant's position in the clap subcommand tree.
    nesting: CommandNesting,
    /// Whether the variant should be represented in the generated contract.
    disposition: CommandDisposition,
}

/// Parses clap command attributes that affect schema registration.
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

/// Parsed semantic metadata that cannot be inferred from Clap or the handler.
#[derive(Default)]
struct VariantSchema {
    /// Explicit semantic input type when the argv carrier differs from it.
    input: Option<Type>,
    /// Optional deprecation notice.
    deprecated: Option<Expr>,
    /// Whether this command should be omitted from the contract.
    skip: bool,
    /// Argument used to transport structured JSON input.
    structured: Option<LitStr>,
    /// Optional value indicating stdin for structured input.
    stdin: Option<LitStr>,
    /// Whether structured input is the only supported input transport.
    structured_only: bool,
    /// Properties that must be encoded as JSON argument values.
    json_properties: Vec<LitStr>,
    /// Explicit semantic-property to clap-argument bindings.
    bindings: Vec<(LitStr, LitStr)>,
}

impl VariantSchema {
    /// Returns whether the variant has no schema metadata at all.
    const fn is_empty(&self) -> bool {
        self.input.is_none()
            && self.deprecated.is_none()
            && !self.skip
            && self.structured.is_none()
            && self.stdin.is_none()
            && !self.structured_only
            && self.json_properties.is_empty()
            && self.bindings.is_empty()
    }

    /// Returns whether the variant contains leaf-only metadata.
    const fn has_leaf_metadata(&self) -> bool {
        self.input.is_some()
            || self.deprecated.is_some()
            || self.structured.is_some()
            || self.stdin.is_some()
            || self.structured_only
            || !self.json_properties.is_empty()
            || !self.bindings.is_empty()
    }
}

/// Parses semantic `#[schema(...)]` metadata from a subcommand variant.
fn parse_variant_schema(attrs: &[Attribute]) -> syn::Result<VariantSchema> {
    let mut result = VariantSchema::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("schema")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("input") {
                result.input = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("deprecated") {
                result.deprecated = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("skip") {
                result.skip = true;
            } else if meta.path.is_ident("structured") {
                result.structured = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("stdin") {
                result.stdin = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("structured_only") {
                result.structured_only = true;
            } else if meta.path.is_ident("json") {
                meta.parse_nested_meta(|property| {
                    let ident = property
                        .path
                        .get_ident()
                        .ok_or_else(|| property.error("json property must be an identifier"))?;
                    result.json_properties.push(LitStr::new(&ident.to_string(), ident.span()));
                    Ok(())
                })?;
            } else if meta.path.is_ident("bind") {
                meta.parse_nested_meta(|binding| {
                    let ident = binding
                        .path
                        .get_ident()
                        .ok_or_else(|| binding.error("binding property must be an identifier"))?;
                    let argument: LitStr = binding.value()?.parse()?;
                    result.bindings.push((LitStr::new(&ident.to_string(), ident.span()), argument));
                    Ok(())
                })?;
            } else {
                return Err(meta.error("unsupported #[schema(...)] command option"));
            }
            Ok(())
        })?;
    }
    Ok(result)
}

/// Returns whether a clap `#[command(...)]` attribute contains the requested flag.
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

/// Returns the payload type of a single-field tuple variant.
fn single_payload_type(fields: &Fields) -> Option<Type> {
    match fields {
        Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
            fields.unnamed.first().map(|field| field.ty.clone())
        }
        _ => None,
    }
}

/// Unwraps an `Option<T>` type used for an optional root subcommand field.
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

/// Resolves the public `clap_schema` path from within the proc macro crate.
fn clap_schema_path() -> TokenStream2 {
    match crate_name("clap_schema") {
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, proc_macro2::Span::call_site());
            quote!(::#ident)
        }
        Ok(FoundCrate::Itself) | Err(_) => quote!(::clap_schema),
    }
}
