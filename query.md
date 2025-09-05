# Sunwet Query Language (probably need a better name for this)

This is not intended to be an omnipotent interface to the graph. The idea is that this should be a convenience for simple queries, while more complex things should be done via a more powerful programmatic interface. It's a work in progress, there are some features I'd like to add, and I'd like to change the uhh, self similarity inflection point to make it more composable.

**Notes**

The documentation refers to values and string values. Values refer to nodes in the database and are described in the values section at the bottom. When the documentation refers to string values, it means a value which can only take the simple quoted string form (or be a parameter that resolves to a string).

The forms uses these conventions:

- Lower case text indicates verbatim text

- Capital text indicates something to be replaced

- `+` means the preceding item must occur one or more times

- `?` means the preceding item can occur zero or one times

- `*` means the preceding item can occur zero or more times

- `(` and `)` are verbatim, but themselves and anything between them are treated as a single item for the purpose of the above modifiers

## The basics

Suppose you have a graph with triples like:

```
...
"xyz123" "sunwet/1/is" "sunwet/1/album"
"xyz123" "sunwet/1/name" "The Hounds of Baskerville"
"xyz124" "sunwet/1/is" "sunwet/1/album"
"xyz124" "sunwet/1/name" "Hounds basking in the sun"
"xyz125" "sunwet/1/is" "sunwet/1/album"
"xyz125" "sunwet/1/name" "Beethoven best 2025"
"xyz126" "sunwet/1/is" "sunwet/1/album"
...
```

```
"sunwet/1/album" -< "sunwet/1/is" { => id }
```

The query syntax follows this form: `ROOT? STEP+ SELECT`

This simple example starts at the node with JSON string value "sunwet/1/album" (the root), moves from object to subject over predicates with the value "sunwet/1/is" (steps), then returns those subjects as `id` in each row of output (selection).

In other words, it returns the ids of all albums (if you're confused, or if you're not confused yet: id isn't a property of a album in this ontology, album is a property of an id).

One row is returned for each element in the result set for the top level query (in this example, one row per album).

Each part is described in more detail below.

## The root

A root value is optional and determines the starting set for the query.

The query can start with no root, in which case the starting set is considered to be every possible value (note you can't actually return every value, you must have at least one movement step if no root is specified).

A query can start with a value root (like the value `"sunwet/1/album"` above) in which case the starting set has just one element, that value.

A query can also start with a search expression: `search EXPR`.

### Search expressions

With a search expression, the search results will become the starting set.

The search is done against a case insensitive trigram full text index (sqlite fts5).

The expression can be any text; the text is split by whitespace, and each element is then joined with `AND` to produce the full text search query. You can quote text to prevent it from being split.

For an example, `hounds bask` might result in the set `"the hounds of baskerville"` `"hounds basking in the sun"`. `"the hounds of"` would result in only `"the hounds of baskerville"`.

If you want to use fts5 syntax yourself, you can prefix your search expression with `raw:` (e.g. `"raw:\"hounds\" AND \"bask\"").

## Steps

Steps (like `<- "sunwet/1/is"`) are executed left to right. They take a set of values and produce a new set via some rule. In the above example, the initial set is a single value `"sunwet/1/album"`. After the movement step the set will be a bunch of ids, where each ID has a link to `"sunwet/1/album"` by the `"sunwet/1/is"` predicate.

### Common options

All steps take common options in the form `STEP DIRECTION? first?`

- `DIRECTION` can be `asc` or `desc`

  This orders the values in the result set. This is typically only useful for the last step, or if using `first` (below)

- `first` (verbatim)

  This trims the result set to just the first element. (It's undefined which element is first unless you specify a sort direction.)

### Move down, move up

- `-> PREDICATE FILTER?`
- `-< PREDICATE FILTER?`

The basic steps are `-> PREDICATE` and `-< PREDICATE`. The first moves from subject to object, the second moves from object to predicate. (Note that this can be confusing, you're always reading in one direction but the movement across the graph can be forward or backward.)

`PREDICATE` is a string value.

`FILTER` optionally filters elements from the result set.

#### Filtering

A filter is a boolean expression tree used to whitelist values added to the result set. You can think of filter expression execution as operating on a single element at a time (the "target" element).

- Exists: `?( STEP* SUFFIX? )`

  This takes the target element and produces a new result set using `STEP*` which can be further filtered with `SUFFIX`. If the result set has at least one element, the target element passes.

  Example: `?( -> "sunwet/1/name" == "hello" )` if used on a set if IDs would result in the set of IDs with an associated "name" with value `"hello"`.

  Suffix can be `==` `!=` `<` `>` `<=` `>=` followed by a value, or `~=` followed by a string "like" expression (a string including `%` to indicate a wildcard, matching SQL `LIKE` syntax)

- Doesn't exist: `!( STEP* SUFFIX? )` - this behaves the same as `?(` but with inverted result

- Or: `|( FILTER+ )` - true if any of the sub-filter expressions are true

- And: `&( FILTER+ )` - true if all of the sub-filter expressions are true

### Recurse

- `-* ( STEP+ )`

This executes `STEP+` and unions it with the starting set to produce a result set. This happens repeatedly until the result set (the running union) no longer changes. The elements in the result set are ordered by recursion depth.

### Junction and/or (intersection/union)

- `-& ( ROOT? STEP+ )`
- `-| ( ROOT? STEP+ )`

Titled "junction and" and "junction or" (intersection and union).

This executes `STEP+` and intersects or unions the results with the input set to produce the result set.

`ROOT` behaves the same as it does at the root of the query (see more details on the root above).

## Selection

Selection has the format `{ BIND? ( SUBQUERY )* }`.

In a top level query, each value in the input set (er, the result set of the last preceding step) will become a row in the output. In a subquery, the entire input set will be turned into an array value and placed in the row with this name. That is, at the moment, there's no way to create nested structures in result rows with subqueries.

`BIND` has the format `=> ID` where `ID` is a simple unquoted string (a string using just `a-zA-Z0-9_-`) and determines the name of the field the value will be placed in in the row.

`SUBQUERY` is the same as the root query, except `shuffle?` can only be used on the root query.

## Values

Values (nodes) can be specified as any of the following:

- Any primitive json value (`"string"`, `true`, `false`, `4` `null`, etc) - except for numbers, which can only be integers or decimals (no exponential notation, no `inf` `nan`)

- Arbitrary json, using `v#*JSON#*` (i.e. a `v` followed by one or more `#`, the JSON itself, then a matching number of `#` to end the value)

- A parameter, like `$author`. Parameters can be supplied by name and will be substituted when executing the query

## Using

- The sunwet UI has a `query` view which allows you to write queries and see their results live

- `sunwet` has command `compile-query` which turns a query into JSON. All API

- `sunwet` has command `query` which takes a compiled query and executes it against the server

- See API documentation for making a query directly via the API
