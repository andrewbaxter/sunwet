# Design notes

## View-based access policies

My original idea was to add permissions to nodes, like "person X can access file Y". I couldn't come up with a decent UX for this, and I was worried about losing data (i.e. you miss permissions on one node, and suddenly you're stuck debugging access issues when it's missing for some user) or accidentally exposing data. Or adding permissions to relations, with similar issues, or even both. Additionally, assuming a fairly connected graph and the fact that nodes are "content addressable" the problem was pretty ambiguous in places too.

Instead, I decided to allow access at the view (query)/form-level. As an admin, you define a view that only allows access to certain data, and then you can selectively provide access to that view.

Full queries/graph viewing/editing are only allowed to the admin user.

I think this is a good compromise that balances flexibility with ease of administration and safety. It means that you probably want a single server per person, rather than colocating multiple peoples' data in the same instance.

## Not URIs

RDF uses URIs for the nodes and predicates. Namespacing and versioning stuff is good, but I decided against it for several reasons:

- Encoding data is hard. Like if you wanted to add a "name" edge with the object being a name - how do you encode that as a URL? It's not obvious, and even if you do know the "correct" way, what about encoding a paragraph of text? Newlines, etc? URL encoding? By contrast, Sunwet uses JSON so you'd just make the name node `"Somebody"`

- RDF proponents suggest URLs with domains - and I didn't want to tie this to the domain name system at all

- It's extremely verbose

## IDs are strings, any value can be an ID

Triples don't make a distinction between content-type data and ids. In early designs I had a distinct "id" type string. I ended up removing it because it complicated the code and I thought it was easier to understand with a simpler model.

The advantage of having an ID type is that tooling could identify IDs and provide better context-aware interactions; for instance, when viewing a query, the UI could show a node link only for ID values, rather than all values on the off chance that one of them is a legitimate ID.

## Custom query language

I originally started this with [cozodb](https://github.com/cozodb/cozo) which used datalog, but doing the sorts of recursive queries I wanted with arbitrary separation between values (i.e. get name at N levels of indirection via X edges) was _extremly_ cumbersome and couldn't be encapsulated/generalized - queries required lots of giant, query-specific boilerplate clauses and any changes were heavily non-local.

Similarly, I looked at SparQL and it didn't seem much less cumbersome.

I have no doubt that the new query method is limited, and this could turn out to be a terrible idea, but at least for the types of queries I can anticipate right now it's pretty good I think.

## CSS-in-JS

I'm not going to describe the common issues in CSS that I'm working around here (and I don't think are sufficiently solved by existing frameworks).

My main goals were:

- Avoid using CSS queries to apply styles to multiple elements in a component. This can accidentally affect nodes deeper in the tree, making debugging/fixing intractable. Instead, each class should basically only affect the exact element it's applied to.

- Define all style in one place (no separate CSS and HTML). Having to jump back and forth, come up with a consistent, unique naming scheme to use on both sides (CSS, HTML) made editing slow and difficult. Instead I wanted to define the CSS for an element inline (while still keeping full selector functionality for pseudo-classes etc).

AFAIK this isn't really solved by CSS frameworks. Components semi-solve this (you still define CSS separately though, I think?).

Anyway, my solution was to make my own "components" in JS with a helper method to lazily define CSS classes. I think this worked pretty well! There's some stuff I'd like to clean up, but overall I could reason about components independently, and troubleshooting issues wasn't hard.
