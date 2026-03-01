# Sunwet Query Language (probably need a better name for this)

This is supposed to be a simple way to get data from the graph - it is not intended to do everything. It's a work in progress, there are some features I'd like to add, and I'd like to change the uhh, self similarity inflection point to make it more composable.

**Notes**

The documentation refers to "values" and "string values". "Values" refer to nodes in the database and are described in the values section at the bottom. When the documentation refers to "string values", it means a value which can only take the simple quoted string form (or be a parameter that resolves to a string).

The syntax documentation uses these conventions:

- Lower case text indicates verbatim text

- Capital text indicates something the querier must replace

- `+` means the preceding item must occur one or more times

- `?` means the preceding item can occur zero or one times

- `*` means the preceding item can occur zero or more times

- `(` and `)` are verbatim, but they themselves as well as anything between them are treated as a single item for the purpose of the above modifiers

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

An example query is:

```
"sunwet/1/album" -< "sunwet/1/is" { => id }
```

The query syntax follows this form: `ROOT? STEP+ STRUCT? SORT?`

The query describes movements from one set to the next, starting with the root or else "everything". In this query, it starts with the set with just the node with JSON string value "sunwet/1/album" (the `ROOT`: `"sunwet/1/album"`), then using that set as "object" values moves to the set of "subject" values over predicates with the value "sunwet/1/is" (the `STEP+`: `-< "sunwet/1/is"` -- `-<` is used because the "is" relation points from "subject" to "object" but we want to move from "object" to "subject"). Since this is the final step, one row is output for each value in set. The `STRUCT?` (`{ => id }`) turns each row from a single value into a struct, and binds the row value to the field `id` in the struct.

Uh, but to summarize what the query actually does, it returns the ids of all albums. (If you're confused, or if you're not confused yet: id isn't a property of "album" in this ontology, "album" is a property of an id).

Each part is described in more detail below.

## The `ROOT`

A root value is optional and determines the starting set for the query.

The query can start with no root, in which case the starting set is considered to be every possible value (note: this is a special case and you can't actually return every value, you must have at least one movement step if no root is specified).

A query can start with a value root (like the value `"sunwet/1/album"` above) in which case the starting set has just one element, that value.

A query can also start with a search expression: `search EXPR`.

### Search expressions

With a search expression, the search results will become the starting set.

The search is done against a case insensitive trigram full text index (sqlite fts5).

The expression can be any text; the text is split by whitespace, and each element is then joined with `AND` to produce the full text search query. You can quote text to prevent it from being split.

For an example, `hounds bask` might result in the set `"the hounds of baskerville"` `"hounds basking in the sun"`. `"the hounds of"` would result in only `"the hounds of baskerville"`.

If you want to use fts5 syntax yourself, you can prefix your search expression with `raw:` (e.g. `"raw:\"hounds\" AND \"bask\"").

## `STEP`s

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

- `-& ( (ROOT? STEP*)* )`
- `-| ( (ROOT? STEP*)* )`

Titled "junction and" and "junction or" (intersection and union).

Each performs a junction on the results of each parenthesized element within the outer parentheses, like `-| ( (-> "a") (-> "b") )` (union the results of following the `a` and `b` predicates from the input set). The incoming set is discarded unless you explicitly include an empty element: `-| ( () (-> "b") )`.

If `ROOT` is specified, the input set to the operator is ignored for that element - `ROOT` behaves the same as it does at the root of the query (see more details on the root above). Otherwise the steps in each element start from the junction's input set.

## `STRUCT`

Selection has the format `{ BIND? ( SUBQUERY )* }`.

`BIND` has the format `=> NAME` where `NAME` is a simple unquoted string (a string using just `a-zA-Z0-9_-`) and determines the name of the field the value will be placed in in the row.

`SUBQUERY` is the same as the root query, except without sorting: sorting can only be used on the root query.

In a top level query, one row is output per value in the last step's output set, so `NAME` will always be a scalar. In subqueries, if the final step doesn't specify `first`, the output set of the last step in the subquery will be turned into an array and placed in `NAME` in the output struct -- at this time there is no way to create nested structs in subqueries.

If no `STRUCT` is specified, the output will be an array consisting of the output set values directly, without being turned into structs and bound to a field (i.e. elements of `"x"` rather than `{"name": "x"}`).

## Values

Values (nodes) can be specified as any of the following:

- Any primitive json value (`"string"`, `true`, `false`, `4` `null`, etc) - except for numbers, which can only be integers or decimals (no exponential notation, no `inf` `nan`)

- Arbitrary json, using `v#*JSON#*` (i.e. a `v` followed by one or more `#`, the JSON itself, then a matching number of `#` to end the value)

- A parameter, like `$author`. Parameters can be supplied by name and will be substituted when executing the query

## Using

- The sunwet UI has a `query` view which allows you to write queries and see their results live

- `sunwet` has command `compile-query` which turns a query into JSON.

- `sunwet` has command `query` which takes a compiled query and executes it against the server

- See API documentation for making a query directly via the API
