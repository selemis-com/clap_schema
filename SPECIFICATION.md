# clap_schema Discovery Contract 0.2

This document specifies the machine-readable discovery contract emitted by `clap_schema` 0.2.x.

The contract is designed for black-box consumers such as agents. A consumer should be able to discover a command, construct its canonical invocation from structured data rather than rendered help, and inspect the JSON Schema of a typed successful result when one is available.

The contract intentionally describes CLI semantics, not Clap's internal representation. Clap remains the source of truth used by the Rust implementation.

## Conventions

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative.

Unknown object properties are allowed. Consumers **MUST** ignore properties they do not understand. This keeps the core contract small and allows applications to add their own metadata.

Properties defined by this contract use lower camel case. Embedded JSON Schema keeps the standardized JSON Schema vocabulary, and application-owned schemas and extension values retain the application's own serialized property names.

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
  "ancestors": [
    {
      "name": "tool",
      "path": []
    },
    {
      "name": "objects",
      "path": ["objects"]
    }
  ],
  "description": "Get an object",
  "invocable": true,
  "output": {}
}
```

### `name`

`name` is the canonical name of the selected command. It does not include parent command tokens.

### `path`

`path` is the ordered canonical command path excluding the executable name. Consumers **MUST** preserve this order when constructing an invocation.

### `ancestors`

`ancestors`, when present, contains the invocation-relevant command levels above the selected command, ordered from the root command to the immediate parent. Root command documents omit this field.

Each ancestor uses the same `arguments`, `options`, `groups`, command-syntax, and subcommand-routing properties defined for the selected command. Its `path` identifies the command boundary that owns those semantics. Global arguments propagated by Clap are represented once at the highest command level where they appear and omitted from descendant levels. This preserves the command boundary of local groups that constrain a global argument while avoiding duplicate copies. Ancestor contexts do not carry `invocable`, `output`, or child topology because they describe how to reach the selected command rather than a separately selected operation.

Consumers constructing a nested invocation **MUST** apply ancestor requirements and routing rules at the command level where they are declared. In particular, selecting a child does not implicitly discard a parent's required arguments unless that ancestor has `subcommandNegatesRequirements: true`.

### `description`

`description`, when present, describes the command for humans and agents. It is descriptive context, not invocation grammar.

### `invocable`

`invocable: true` means the exact selected command path can terminate as an application operation. Builder registrations that target a Clap command with `subcommand_required(true)` are rejected because that path cannot terminate without selecting a child.

A command can be both invocable and have optional subcommands. Absence of `invocable` is equivalent to `false`.

`invocable` does not refer to an operating-system executable or binary.

### `arguments`

`arguments` contains positional arguments accepted by the reflected Clap command.

The array is ordered by invocation position. Each positional also carries an explicit one-based `position`; consumers **MUST** supply positionals in ascending `position` order.

### `options`

`options` contains non-positional arguments accepted by the reflected Clap command. Each option is emitted with exactly one canonical spelling. A long spelling such as `--limit` is preferred when available; otherwise the canonical short spelling such as `-v` is used.

Alternative short names and aliases are intentionally not part of the core contract.

### `groups`

`groups`, when present, describes Clap argument groups that contain reflected arguments and materially affect invocation validity through requiredness or mutual-exclusion cardinality. Unconstraining groups are omitted.

See [Argument groups](#argument-groups).

### `allowMissingPositionals`

`allowMissingPositionals: true` means a positional may be omitted while a later positional is supplied when Clap can disambiguate the remaining values. Consumers MUST preserve explicit positional `position` values rather than compacting later values into earlier slots.

### `dontDelimitTrailingValues`

`dontDelimitTrailingValues: true` means values parsed after `--`, or captured by a trailing variadic positional, are not split by configured value delimiters.

### `argsConflictWithSubcommands`

`argsConflictWithSubcommands: true` means arguments belonging to the selected command cannot be combined with selecting one of its child subcommands. Consumers choosing a child MUST NOT also construct parent-command arguments at that same command level.

### `subcommandPrecedenceOverArg`

`subcommandPrecedenceOverArg: true` means a recognized child command token terminates greedy value consumption by an argument and is parsed as the subcommand instead.

### `subcommandNegatesRequirements`

`subcommandNegatesRequirements: true` means selecting a valid child subcommand waives otherwise-required arguments on the selected parent command. The requirements still apply when the parent is invoked without a child.

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
    "minValues": 1,
    "maxValues": 1,
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

`required: true` means Clap's base required setting is enabled for the argument. This base
requiredness is evaluated together with reflected conflicts; for example, a conflict can make an
otherwise-required argument inapplicable for a particular invocation.

Absence is equivalent to `false`.

### `global`

`global: true` means Clap propagates this argument to child commands. A global argument may be supplied at the command level where it is declared or at any descendant level accepted by Clap.

In a complete nested command document, a propagated global argument is represented once at the highest command level where it appears and omitted from descendant levels. Consumers SHOULD emit it at that represented command level. This is the canonical placement because ancestor-local argument groups can constrain the global argument at that boundary even when Clap also recognizes the option after a descendant subcommand.

Absence is equivalent to `false`.

### `value`

`value` is present when one occurrence of the argument consumes one or more values. Its absence means the argument is a flag-like token that consumes no separate value.

### `repeatable`

`repeatable: true` means the argument's reflected action is intended to be supplied more than once, such as an append or count action.

A canonical consumer SHOULD repeat an argument only when `repeatable` is true.

### `conflictsWith`

`conflictsWith` contains canonical argument names in argument-level conflict relationships with this argument. These relationships are normalized symmetrically, matching Clap's two-way conflict semantics.

A consumer MUST NOT construct an invocation containing an argument together with a listed conflict.

### `requireEquals`

For a value-taking option, `requireEquals: true` means the option and its first value must use `=` syntax:

```text
--color=always
```

rather than:

```text
--color always
```

### `requiresDoubleDash`

For a positional, `requiresDoubleDash: true` means the positional must occur after the option terminator:

```text
-- <value>
```

### `trailingVarArg`

For a positional, `trailingVarArg: true` means that once this positional begins consuming values, the remaining command-line tokens are treated as values for it rather than being parsed as options or subcommands.

### `exclusive`

`exclusive: true` means the argument must be used without any other arguments for the selected command.

## Value document

A value-taking argument contains a `value` document:

```json
{
  "minValues": 1,
  "maxValues": 1,
  "values": ["active", "archived"],
  "default": "active",
  "delimiter": ",",
  "terminator": ";",
  "allowHyphenValues": true
}
```

### `minValues` and `maxValues`

`minValues` and `maxValues` describe how many values one occurrence consumes.

`maxValues: null` means the upper bound is unbounded.

For example:

```json
{
  "minValues": 1,
  "maxValues": null
}
```

means one or more values.

These fields apply per occurrence. `repeatable` separately describes whether the argument itself is intended to occur multiple times.

### `values`

`values`, when present, contains canonical possible values advertised by the configured parser. Consumers SHOULD prefer one of these values when constructing an invocation, but MUST NOT treat the array as proof that other values are invalid: Clap exposes possible values as reflection metadata and a parser may advertise values without making that list exhaustive. Hidden aliases are not emitted as additional canonical values.

Clap help/completion visibility does not change this field. Canonical possible values hidden with Clap's presentation controls remain part of the machine-readable contract.

### `default`

`default`, when present, is the lexical default used when the argument is omitted.

A single default is a JSON string. Multiple defaults are an array of JSON strings. Defaults remain lexical because Clap parses them into application values after command-line processing.

Clap's `hide_default_value` setting affects human help only and does not suppress the default from this machine-readable contract.

### `delimiter`

`delimiter`, when present, is the character used to split multiple values inside one command-line token.

### `terminator`

`terminator`, when present, is the token that stops consumption by a multi-valued argument.

### `allowHyphenValues`

`allowHyphenValues: true` means values beginning with `-` can be consumed as values without otherwise disambiguating them from options.

### `allowNegativeNumbers`

`allowNegativeNumbers: true` means negative-number tokens may be consumed as values without being interpreted as options.

### `ignoreCase`

`ignoreCase: true` reflects Clap's case-insensitive matching for advertised possible values.

## Argument groups

A group document has the following semantic shape:

```json
{
  "name": "input",
  "members": ["--stdin", "--file"],
  "required": true,
  "multiple": false
}
```

`name` is the stable group identifier. `members` contains canonical argument names.

`required: true` means Clap's base required setting is enabled for the group. Absence is equivalent to `false`.

`multiple: true` means more than one member may be present. Absence is equivalent to `false`, so a group with multiple members is mutually exclusive by default. When the group requirement applies, combining `required: true` with `multiple: false` means exactly one member must be present.

## Shallow subcommand summaries

In shallow discovery, a direct child can be represented as:

```json
{
  "path": ["objects"],
  "description": "Manage objects",
  "hasSubcommands": true
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
- optional `hasSubcommands`.

`hasSubcommands` is retained on shallow summaries because child documents are intentionally not expanded there. In a complete command document, topology is represented by the actual `subcommands` array instead.

## Canonical invocation construction

Given the executable name separately and one complete command document, a consumer can construct a canonical invocation as follows:

1. Start with the executable name.
2. Walk `ancestors` from root to immediate parent. At each ancestor level, construct that level's represented options and positional values according to its local argument, group, and syntax properties, then append the next canonical token from the selected command's `path`. A global argument is emitted at the highest level where the contract represents it. Respect that ancestor's `argsConflictWithSubcommands`, `subcommandPrecedenceOverArg`, and `subcommandNegatesRequirements` while crossing the command boundary.
3. After the final path token, construct the selected command's own represented options and positional values from the top-level command properties. Globals already represented by an ancestor are omitted here.
4. For a selected option with `value`, supply a number of values within `minValues..=maxValues`. When `maxValues` is `null`, there is no finite upper bound.
5. If `requireEquals` is true, attach the first option value with `=`.
6. Respect `delimiter`, `terminator`, `repeatable`, `conflictsWith`, `ignoreCase`, and `exclusive` when they are present.
7. Satisfy every applicable argument-group cardinality rule at each command level.
8. Append positional values at each command level according to their explicit `position`, respecting `allowMissingPositionals`. If any positional has `requiresDoubleDash: true`, insert `--` before that positional as required by that command level. Once a `trailingVarArg` positional begins consuming values, treat the remaining tokens as its values; when `dontDelimitTrailingValues` is true, do not split those trailing values by configured delimiters.
9. When `values` is present, prefer one of the advertised values, but do not treat that list as exhaustive validation. Every supplied lexical value must still satisfy Clap's configured parser.
10. When relying on defaults, use the reflected unconditional lexical `default` when it is present.

The schema does not prescribe shell quoting or escaping. The caller is responsible for passing the resulting argument vector safely to the process. Agents and tools SHOULD prefer direct argv/process APIs over constructing a shell command string.

## Reflection boundary

`clap_schema` 0.2 describes semantics that can be obtained reliably from Clap's public built-command reflection plus the registered Rust output type.

The core contract includes canonical paths and option spellings, global argument scope, positional order, base requiredness, argument-group cardinality, value arity, unconditional defaults, advertised possible values, delimiters, value terminators, repeatability, conflicts, exclusive arguments, required `=` syntax, required `--` syntax, trailing variadic capture, case-insensitive value matching, missing-positional behavior, trailing-value delimiting, parent/subcommand routing semantics, and typed successful-output JSON Schema.

Input values remain lexical command-line values. `clap_schema` does not infer Rust result types from Clap's erased value parser, and Clap remains authoritative for parser-specific validation.

Parser configuration that is not exposed through Clap's built-command reflection is not inferred. This includes `args_override_self`; consumers should use the canonical argument occurrence semantics represented by the contract rather than treating every argv form accepted by Clap as discoverable.

Only UTF-8 lexical defaults can be represented directly by this JSON contract. A non-UTF-8 default is omitted rather than converted lossily. Consumers MUST interpret omission as “not stated by this contract”, not as proof that no application-specific default exists.

The contract uses normal process-style argv framing at the root parser entrypoint, where the executable name is separate from the command path. `ContractBuilder::build` rejects root `no_binary_name` and `multicall` modes because they change the meaning of the beginning of argv. Equivalent settings on nested commands remain valid because they do not change root process framing. Runtime external-subcommand capture and parser control flow such as `arg_required_else_help` remain Clap-authoritative outside the structured contract.

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

## Presentation visibility

Clap presentation settings such as hidden commands, hidden arguments, hidden defaults, and hidden possible values do not remove parser behavior from the machine-readable contract. `clap_schema` reflects the command interface accepted by Clap rather than reproducing human help visibility.

## Compatibility

0.2 is a breaking wire-format revision from 0.1.

Notable 0.2 changes include:

- `executable` is replaced by `invocable`;
- rendered `usage` is removed;
- command aliases are not emitted;
- `hasSubcommands` is removed from complete command documents and retained only where needed by shallow summaries;
- argument `id`, `index`, `short`, `long`, `value_names`, `help`, `default_values`, and `possible_values` are replaced by the canonical semantic argument/value shape specified here;
- input value arity is explicit while values remain lexical;
- global argument scope is explicit and propagated globals are represented once at their highest command level;
- syntax-affecting details such as required `=`, required `--`, delimiters, terminators, repeatability, conflicts, and exclusivity are exposed directly;

Within the 0.2 line, adding an optional field is compatible. Consumers MUST ignore unknown fields.

Removing a core field, renaming a core field, or changing the meaning of an existing field is a breaking wire-format change.
