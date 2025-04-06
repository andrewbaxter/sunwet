Sunwet is a combined file manager and graph database with media playback capabilities. In short, use it to organize anything! Notes, music, art, photos, video, software, comics, archived websites, scanned documents, financial transactions, appointments, people you know, emails, passwords (encrypted!), etc etc.

Is this a good idea? Read more to find out.

# On organizing things

AFAIK there's basically two common ways for organizing digital data:

## A hierarchy

This is like the filesystem, where you have folders, and maybe folders within folders, and files in those folders. For organizing music, you might have a `music` folder, within which there's a folder for each artist, within which there's a folder for each album by that artist, then within that the songs on the album (named starting with the track number, then the track title).

This works great if you know the artist, but if you want to look up an album by genre, or by year released, you can't (easily - in order to do this you'd have to go through every single folder in the entire hierarchy!).

So more commonly you probably see...

## Tags

Also known as: labels, sets, or a graph with a maximum depth of 1.

Basically, every item has one or more tags, and you can look up items by doing an arbitrary union and disunion of the sets defined by the tags: You might have a tag named `music`, a tag named `artist=Somebody`, a tag named `album=RGB`, a tag named `release_year=1901`. If you want to look up tracks from an artist, you can do a query to intersect the `artist=Somebody` and `album=RGB` sets to get just the tracks from the album "RGB" by "Somebody". Unlike with a hierarchy though, you could just as easily look for albums by year by looking at the `release_year=1901` set.

Again though, this has limitations:

- What if you want to do queries on tags? Like get a list of albums?

- Suppose you have collections of cat and dog photos. You want to look up photos with cats with brown fur, so you query for the intersection of "cat" and "fur_color=brown" -- but wait, this returned photos where the photos have only white cats and brown dogs!

Anyways, sometimes you have combinations of tags and hierarchies. I think Obsidian has both tags and folders, separately, for instance, and there are other systems with tag hierarchies or multi-hierarchies, but in the end these have the same limitations as hierarchies.

Also some software implement workarounds for specific instances of the limitations above, like listing artists in a music library manager - but it's not enabled generically by the internal tagging. If you have some nonstandard data to classify where no specific workaround has been implemented you're out of luck.

## Knowledge graphs

So there are "knowledge graphs" where you encode knowledge... in a graph. RDF and the semantic web is a better known form of this, but I've seen discussions of hypergraphs and other forms of knowlege organization.

Taking the above example about cats and dogs, you could solve this by having a number of "subject" edges, where one "subject" has edges "is -> cat" and "has fur -> white", and another "is -> dog" and "has fur -> brown", then write a query like "(the photo or any subject) has edges "is -> cat" and "has fur -> white".

Do knowledge graphs work? I have no idea, I made this, as an accessible and general-purpose knowledge graph, to find out.

# On not-organizing things

Some people say organizing things like this requires unrealistic diligence, it's not worth it - doing full text search is all you really need.

I agree there're a lot of uses that full text search works great for, but there're also a lot of uses it doesn't:

- Nobody searches their music collection using full text search, even if the relevant text is in the track metadata. They find music by categories - genre, artist, purchase date, mood. And more importantly, they frequently aren't _searching_ but _exploring_.

  To expand on this, Wikipedia and many websites are full of graph data. I often go to wikipedia to find "the name of the song on the album that came out after the other album by this artist" where all I can remember is that the artist did a collaboration with this other artist. I find that by looking up the other artist, then following the links (graph) wheras for full text search to help there'd need to be one "text" that contains all the links I'd want to search.

- You can sometimes find lists of things (gtk themes) and relations online (movies starring Keanu Reeves) via full text search (aka Google) but that's because the internet is full of monkeys producing every possible combination of data which Google then indexes. Local search won't magically produce a list of items you want unless you made the list before hand or have accompanying text that includes all keywords you might want.

- Full text search can only deal with the binary of present/not present. If you want a list of ablums released in 1980-1990: full text search won't help you with ranges, but this is trivial with structured data-based queries.

- Having data organized well makes it easy for software to consume - home assistants, importers, viewers, etc. You'd never have tax software generate a tax report by searching for transactions using full text search.

Also, while some of this data might be added by hand, I'd expect a lot of it to be added using automatic importers, perhaps reading from other databases of well-classified information.

Sunwet supports full text search too though.

Edit: Actually today people would probably say just use AI... I wrote this a couple years ago originally. I'm not going to address this directly, but to some point I think the above still applies: in order to train AI you still need structured, and I think AI (chat) wouldn't be a satisfactory interface for many interactions above.

# What is Sunwet

Sunwet is:

- A graph database, with a simple query language

- Where some of the nodes reference files, which sunwet stores and serves as long as the node exists

- That has a web UI for formatting and the results of queries, adding data via template forms, and doing trivial editing and

- An API for interacting with it from other software

The graph data stores "triples" like RDF: each triple has a subject, predicate, and object like `UUID-ABCD-EFGH` `sunwet/1/is` `sunwet/1/album`. Unlike RDF it doesn't really bake any web technology in like URIs, and nodes can be any JSON data.

Some extra features:

- Full text search

- Append only, with undo to any change

- Does garbage collection on old changes

- All-JSON API, not even queries use a text DSL

- Optional [FDAP](https://github.com/andrewbaxter/openfdap) user management, with [OIDC](https://github.com/andrewbaxter/fdap-oidc) login

- Query/form-based permissions (not data-based)

# Installation and running

If you clone the repo and install Rust/Cargo, you can do `cargo build` and find the binary in `.cargo_target/debug/sunwet`.

Run the server with `sunwet run-server config.json`.

See [the config schema](config.schema.json) for what goes in the config.

# Reading (queries)

Imagine you start with a working set composed of a single node, like `"sunwet/1/album"`. There are three main operations you can do:

- Move along an edge (predicate) such as `"sunwet/1/is"`, replacing the current working set with all nodes on the other side of the predicate from the first set

- Return the set with a specific name (`id`)

- Do a subquery - that is, keep the current set but perform more operations on a new set based on the first set (like then traverse `"sunwet/1/name"` edges)

There are various other things you can do as well: you can filter at any point, keeping only one value in a set, or removing elements based on criteria. You can recurse (repeat) a set of operations.

A concrete example is:

```
"sunwet/1/album" <- "sunwet/1/is" {
    => id
    (-> "sunwet/1/name" first {
        => name,
    })
}
```

So it starts with `"sunwet/1/album"`, moves _backwards_ through triples with the predicate `"sunwet/1/is"` (from object to subject), and returns those (the subjects) as `id`. Then it does a subquery forward (subject to object) from each id, following `"sunwet/1/name"` edges, and returning the first object as `name`. The output will produce data like:

```
[
    {
        "id": "UUID-ABCD-EFGH",
        "name": "Something"
    }
]
```

# Writing (commits)

Changes are basically a list of triples to add and remove.

Once a

## Full reference:

The query is the form: `ROOT OPERATION*`

Nodes are JSON.

`ROOT` is any node

`OPERATION` is:

- `-> PREDICATE` where predicate is a quoted string

- TODO

# Design decisions

## Not URIs

RDF uses URIs for the nodes and predicates. Namespacing and versioning stuff is good, but I decided against it for several reasons:

- Encoding data is hard. Like if you wanted to add a "name" edge with the object being a name - how do you encode that as a URL? It's not obvious, and even if you do know the "correct" way, what about encoding a paragraph of text? Newlines, etc? URL encoding? By contrast, Sunwet uses JSON so you'd just make the name node `"Somebody"`

- RDF proponents suggest URLs with domains - and I strongly dislike the centralized authority of the domain name system

- It adds a ton of noise

## Custom query language

I originally started this with [cozodb](https://github.com/cozodb/cozo) which used datalog, but doing the sorts of recursive queries I wanted with arbitrary separation between values (i.e. get name at N levels of indirection via X edges) was _extremly_ cumbersome and couldn't be encapsulated/generalized - making larger queries required changes throughout the query and not localized to some clause.

Similarly, I looked at SparQL and it didn't seem much less cumbersome.

I have no doubt that the new query method is limited, and this could turn out to be a terrible idea, but at least for the types of queries I can anticipate right now it's capable and succinct.
