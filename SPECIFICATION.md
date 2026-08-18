# clap_schema Discovery Contract 0.2

This document specifies the machine-readable discovery contract emitted by `clap_schema` 0.2.x.

The contract is designed for black-box consumers such as agents. A consumer should be able to discover a command, construct its canonical invocation from structured data rather than rendered help, and inspect the JSON Schema of a typed successful result when one is available.

The contract intentionally describes CLI semantics, not Clap's internal representation. Clap remains the source of truth used by the Rust implementation.

## Conventions

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

Unknown object properties are allowed. Consumers **MUST** ignore properties they do not understand. This keeps the core contract small and allows applications to add their own metadata.

## Discovery

A schema request selects a command path. The selected command is always returned as a complete command document.

In shallow discovery, `subcommands` contains compact summaries of direct children. In full discovery, each child is recursively expanded into the same complete command-document shape.

Command paths are arrays of canonical command tokens and exclude the executable name:

```json
{
  "path": ["objects", "get"]
}
```

The empty path selects the root command.

Implementations MAY accept command aliases when resolving a discovery request, but emitted paths **MUST** use canonical command names.

## Command document

A complete command document can contain:

```json
{
  "name": "get",
  "path": ["objects", "get"],
  "description": "Get an object",
  "invocable": true,
  "output": {}
}
```

### `name`

`name` is the canonical name of the selected command. It does not include parent command tokens.

### `path`

`path` is the ordered canonical command path excluding the executable name. Consumers **MUST** preserve this order when constructing an invocation.

### `description`

`description`, when present, describes the command for humans and agents. It is descriptive context, not invocation grammar.

### `invocable`

`invocable: true` means the exact selected command path can be invoked as an operation.

A command can be both invocable and have subcommands. Absence of `invocable` is equivalent to `false`.

`invocable` does not refer to an operating-system executable or binary.

### `arguments`

`arguments` contains visible positional arguments.

The array is ordered by invocation position. Each positional also carries an explicit one-based `position`; consumers **MUST** supply positionals in ascending `position` order.

### `options`

`options` contains visible non-positional arguments. Each option is emitted with exactly one canonical spelling. A long spelling such as `--limit` is preferred when available; otherwise the canonical short spelling such as `-v` is used.

Alternative short names and aliases are intentionally not part of the core contract.

### `groups`

`groups`, when present, describes Clap argument groups that contain visible arguments and materially affect invocation validity. Group references used by argument relationships resolve against this array. Unconstraining groups that are neither required, mutually exclusive, related to another target, nor referenced by a relationship are omitted.

See [Argument groups](#argument-groups).

### `output`

`output`, when present, is a JSON Schema Draft 2020-12 schema describing the typed successful result registered for the command.

Absence of `output` means **no typed successful-output contract is declared**. It does not assert that the process writes nothing to stdout or that the command has no application-specific output.

The runtime representation of successful values is application-owned. Applications that expose these contracts to agents SHOULD provide a machine-output mode whose successful value matches `output`.

## Argument document

Positionals and options use the same argument shape:

```json
{
  "name": "--limit",
  "description": "Maximum number of items",
  "value": {
    "type": "integer",
    "min_values": 1,
    "max_values": 1,
    "default": "50"
  }
}
```

Fields that are false, empty, or not applicable are generally omitted.

### `name`

For an option, `name` is the exact canonical token to place on the command line, including leading dashes.

For a positional, `name` is its stable semantic identifier. It is not itself emitted as a command-line token; its value is emitted at `position`.

### `position`

`position` is present only for positional arguments and is one-based.

The `arguments` array order and `position` carry the same ordering intentionally. Consumers MUST NOT reorder positional arguments.

### `description`

`description`, when present, is semantic help reflected from the command definition.

### `required`

`required: true` means the argument is unconditionally required by the reflected command model.

Absence is equivalent to `false`.

See [Reflection boundary](#reflection-boundary) for conditional requirements.

### `value`

`value` is present when one occurrence of the argument consumes one or more values. Its absence means the argument is a flag-like token that consumes no separate value.

### `repeatable`

`repeatable: true` means the argument's reflected action is intended to be supplied more than once, such as an append or count action.

A canonical consumer SHOULD repeat an argument only when `repeatable` is true.

### `conflicts_with`

`conflicts_with` contains canonical argument names that cannot be used together with this argument.

A consumer MUST NOT construct an invocation containing an argument together with a listed conflict.

### `overrides`

`overrides` contains canonical argument names whose effective value is replaced when this argument is also present. Consumers SHOULD avoid supplying both unless the overriding behavior is intentional.

### `requires`

`requires` contains requirements introduced by this argument. Each entry has a `when` predicate and a `target`. `when` is either `present` or an equality predicate on this argument's lexical value. `target` identifies either a canonical argument or a named argument group.

When a predicate matches, the referenced target MUST be satisfied. An argument target is satisfied by supplying that argument. A group target is satisfied by supplying a member in accordance with that group's cardinality rules.

### `required_if_any` and `required_if_all`

Each entry is an equality condition on another canonical argument. The array is one aggregate rule; consumers MUST preserve whether that rule uses `any` or `all`.

For `required_if_any`, the argument is required when **at least one** listed condition matches. The listed conditions are therefore combined with logical OR.

For `required_if_all`, the argument is required only when **every** listed condition matches. The listed conditions are therefore combined with logical AND.

These rules are independent of unconditional `required`.

### `required_unless_any` and `required_unless_all`

Each entry identifies an argument or group whose presence can satisfy an exception to requiredness. The array is one aggregate rule; consumers MUST preserve whether that rule uses `any` or `all`.

For `required_unless_any`, the argument is required unless **at least one** listed target is present. Equivalently, the argument becomes optional when any listed target is present.

For `required_unless_all`, the argument is required unless **every** listed target is present. Equivalently, the argument becomes optional only when all listed targets are present.

### `require_equals`

For a value-taking option, `require_equals: true` means the option and its first value must use `=` syntax:

```text
--color=always
```

rather than:

```text
--color always
```

### `requires_double_dash`

For a positional, `requires_double_dash: true` means the positional must occur after the option terminator:

```text
-- <value>
```

### `exclusive`

`exclusive: true` means the argument must be used without any other arguments for the selected command.

## Value document

A value-taking argument contains a `value` document:

```json
{
  "type": "string",
  "min_values": 1,
  "max_values": 1,
  "values": ["active", "archived"],
  "default": "active",
  "delimiter": ",",
  "terminator": ";",
  "allow_hyphen_values": true
}
```

### `type`

`type` is the scalar class that `clap_schema` can reliably infer from the configured value parser:

- `string`
- `integer`
- `number`
- `boolean`

CLI values are lexical tokens. Custom parser output types that cannot be classified reliably are represented as `string` rather than guessed from Rust type names. `type: "string"` therefore does not imply that every string is semantically valid; applications may apply additional parsing or validation.

### `min_values` and `max_values`

`min_values` and `max_values` describe how many values one occurrence consumes.

`max_values: null` means the upper bound is unbounded.

For example:

```json
{
  "min_values": 1,
  "max_values": null
}
```

means one or more values.

These fields apply per occurrence. `repeatable` separately describes whether the argument itself is intended to occur multiple times.

### `values`

`values`, when present, contains visible finite values advertised by the configured parser. Consumers SHOULD prefer one of these values when constructing an invocation.

Hidden possible values are not emitted.

### `default`

`default`, when present, is the visible lexical default used when the argument is omitted.

A single default is a JSON string. Multiple defaults are an array of JSON strings. Defaults remain lexical because Clap parses them into application values after command-line processing.

Hidden defaults are not emitted.

### `default_missing`

`default_missing`, when present, is the lexical value Clap supplies when the argument itself is present but no explicit value is supplied. This is distinct from `default`, which applies when the argument is omitted.

### `default_if`

`default_if` is an ordered array of conditional-default rules. Each rule identifies another canonical argument, a presence or equality predicate, and a lexical default. Rules are evaluated in declaration order; the first matching rule wins. A JSON `null` value explicitly clears an unconditional default when the rule matches.

### `delimiter`

`delimiter`, when present, is the character used to split multiple values inside one command-line token.

### `terminator`

`terminator`, when present, is the token that stops consumption by a multi-valued argument.

### `allow_hyphen_values`

`allow_hyphen_values: true` means values beginning with `-` can be consumed as values without otherwise disambiguating them from options.

### `allow_negative_numbers`

`allow_negative_numbers: true` means negative-number tokens may be consumed as values without being interpreted as options.

## Argument groups

A group document has the following semantic shape:

```json
{
  "name": "input",
  "members": ["--stdin", "--file"],
  "required": true,
  "multiple": false,
  "requires": [{"kind": "argument", "name": "--format"}],
  "conflicts_with": [{"kind": "argument", "name": "--legacy"}]
}
```

`name` is the stable group identifier used by relationship references. `members` contains canonical visible argument names.

`required: true` means at least one group member must be present. Absence is equivalent to `false`.

`multiple: true` means more than one member may be present. Absence is equivalent to `false`, so a group with multiple visible members is mutually exclusive by default. Combined with `required: true`, `multiple: false` means exactly one member must be present.

`requires` and `conflicts_with` apply when the group is present and may refer to either arguments or other groups. A group that would otherwise impose no constraint is still emitted when another reflected relationship targets it.

## Shallow subcommand summaries

In shallow discovery, a direct child can be represented as:

```json
{
  "path": ["objects"],
  "description": "Manage objects",
  "has_subcommands": true
}
```

or:

```json
{
  "path": ["whoami"],
  "description": "Show the current identity",
  "invocable": true
}
```

A summary contains:

- `path`: canonical child path;
- optional `description`;
- optional `invocable`;
- optional `has_subcommands`.

`has_subcommands` is retained on shallow summaries because child documents are intentionally not expanded there. In a complete command document, topology is represented by the actual `subcommands` array instead.

## Canonical invocation construction

Given the executable name separately and one complete command document, a consumer can construct a canonical invocation as follows:

1. Start with the executable name.
2. Append every token in `path`, in array order.
3. Append selected options using their exact `name`.
4. For a selected option with `value`, supply a number of values within `min_values..=max_values`. When `max_values` is `null`, there is no finite upper bound.
5. If `require_equals` is true, attach the first option value with `=`.
6. Respect `delimiter`, `terminator`, `repeatable`, `conflicts_with`, `overrides`, `requires`, conditional requiredness, and `exclusive` when they are present.
7. Satisfy every applicable argument-group cardinality, requirement, and conflict rule.
8. Append positional values in ascending `position` order. If any positional has `requires_double_dash: true`, insert `--` before that positional as required by the command contract.
9. When `values` is present, prefer one of the advertised values. Values must also satisfy the parser represented by `type` and any application-specific validation.
10. When relying on defaults, distinguish omission (`default`) from selecting an option without an explicit value (`default_missing`) and apply ordered `default_if` rules before the unconditional default.

The schema does not prescribe shell quoting or escaping. The caller is responsible for passing the resulting argument vector safely to the process. Agents and tools SHOULD prefer direct argv/process APIs over constructing a shell command string.

## Reflection boundary

`clap_schema` 0.2 describes semantics that can be obtained reliably from Clap's public built-command reflection plus the registered Rust output type.

The core contract includes canonical paths and option spellings, positional order, unconditional and conditional requiredness, requirement and override relationships, argument-group rules, value arity, visible unconditional/missing/conditional defaults, advertised finite values, delimiters, value terminators, repeatability, conflicts, exclusive arguments, required `=` syntax, required `--` syntax, and typed successful-output JSON Schema.

The remaining important boundary is arbitrary value-parser validation. Clap exposes the erased parser's result `TypeId` and advertised possible values, but a custom parser can enforce constraints that are not reflectable as structured data. `type: "string"` in particular can therefore represent an application-defined parser rather than an unconstrained string.

Only UTF-8 lexical relationship and default values can be represented directly by this JSON contract. A rule whose lexical predicate cannot be represented as UTF-8 is omitted rather than converted lossily. Consumers MUST interpret omission as “not stated by this contract”, not as proof that no application-specific constraint exists.

This boundary is intentional: `clap_schema` does not guess parser behavior or make private Clap implementation details part of its wire protocol.

## Extensions

Applications may add fields to command discovery documents. Core consumers MUST ignore unknown fields.

`clap_schema` also supports application-owned typed extension schemas. Their field names and semantics are deliberately not standardized by this specification.

Examples of application concerns that belong outside the core contract include:

- structured error envelopes;
- mutation or idempotency classifications;
- authorization requirements;
- confirmation policies;
- pagination semantics;
- structured-input conventions;
- output projection such as `--fields`;
- domain-specific metadata.

The core contract does not require an application to adopt any of these behaviors.

## Hidden commands and arguments

Clap-hidden commands and arguments are not part of discovery. A consumer MUST NOT infer hidden paths or options from omissions.

## Compatibility

0.2 is a breaking wire-format revision from 0.1.

Notable 0.2 changes include:

- `executable` is replaced by `invocable`;
- rendered `usage` is removed;
- command aliases are not emitted;
- `has_subcommands` is removed from complete command documents and retained only where needed by shallow summaries;
- argument `id`, `index`, `short`, `long`, `value_names`, `help`, `default_values`, and `possible_values` are replaced by the canonical semantic argument/value shape specified here;
- input value arity and scalar type are explicit;
- syntax-affecting details such as required `=`, required `--`, delimiters, terminators, repeatability, conflicts, and exclusivity are exposed directly;
- conditional requirements, required-unless rules, overrides, missing/conditional defaults, and argument-group constraints are reflected as structured semantics.

Within the 0.2 line, adding an optional field is compatible. Consumers MUST ignore unknown fields.

Removing a core field, renaming a core field, or changing the meaning of an existing field is a breaking wire-format change.
