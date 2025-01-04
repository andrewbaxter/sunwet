Sunwet is a combined file manager and graph database with media playback capabilities. In short, use it to organize anything!

- Notes

- Music

- Images

- Video

- Software

- Comics

- Archived websites

- Scanned documents

- Financial transactions

- Appointments

- People you know

- Emails

- Passwords (encrypted!)

Is this a good idea? Read more to find out.

# An essay: Organizing things

AFAIK there's basically two common ways for organizing digital data:

## A hierarchy

This is like the filesystem, where you have folders. For organizing music, you might have a `music` folder, within which there's a folder for each artist, within which there's a folder for each album by that artist, then within that the songs (named starting with the track number, then the track title).

This works great if you know the artist, but if you want to look up an album by genre, or by year released, you can't (easily - you'd have to go through all folders to make sure you didn't miss any).

## Sets, or a graph with max depth 1

You've probably seen these before as "tags" or "labels".

Basically, every item belongs to one or more sets. You might have a set named `music`, a set named `artist=Somebody`, a set named `album=RGB`, a set named `release_year=1901`. If you want to look up tracks from an artist, you can do a query to intersect the `artist=Somebody` and `album=RGB` sets to get just the tracks from the album "RGB" by "Somebody". Unlike with a hierarchy though, you could just as easily look for albums by year by looking at the `release_year=1901` set.

Again though, this has limitations:

- What if you want to do queries on tags? Like get a list of albums?

- What about composing tags? A song has an "artist", but what if the lyrics were done by one person, the composition by another, and performance by a third? Then you have `performer` `lyricist` and `composer` tags. What if there are multiple performers? One playing drums, one playing guitar, one gargling water? Do you make tags `drum_player` `guitar_player` and `gargler`? Then how do you query for all of them together?

- Say you have some albums, then you have a multi-CD album, then an albumset on a theme. If you only have "albums" in your data, then you can't get all the CDs from the albumset. But if you instead classify all the CDs in the albumset as one album, then you can't look up just one disk, and then the track numbers no longer match the CD track numbers. You could classify the tracks as being in an "album" _and_ an "albumset", but then if you query by "albumsets" you don't get albums, and if you query by "albumsets" and "albums" it'll duplicate results from the individual disks in the albumset.

- Something else, I actually ran into the limitations of tags years ago when I decided to make this and I don't remember what those were any more.

Anyways, sometimes you have combinations of tags and hierarchies. I think Obsidian has both tags and folders, separately, for instance, and there are other systems with tag hierarchies or multi-hierarchies, but these have the same limitations as hierarchies.

Also some issues have one-off features solving that problem in a piece of software, like listing artists in a music library manager - but it's not enabled generically by the internal tagging. If you have some nonstandard data to classify, even if the software supports it, it may not be able to do the queries you want.

## Knowledge graphs

So I stumbled upon knowledge graphs. RDF and the semantic web is a better known form of this, but I've seen discussions of hypergraphs and other forms of knowlege organization.

Taking the above example about garglers, you could have a node for each artist involved with the track, related to the track by an `artist` edge. That artist node could have an edge to the person's node, with edges to their name, background, aliases, etc. Then you could additionally add edges to the artist node about their role, instrument, etc.

Then you could query for "tracks with a drummer" "tracks where Someone played guitar" "tracks where Someone was involved in any role" etc.

Do knowledge graphs work? I have no idea, I'm trying this out for the first time myself.

# An essay: Not organizing things

Some people say organizing things like this requires unrealistic diligence, it's not worth it - doing full text search is all you really need.

I agree there's a lot of searches that full text search works great for, but there's also a lot of things it doesn't:

- Nobody searches their music collection using full text search, even if the relevant text is in the track metadata. They find music by categories - genre, artist, purchase date, mood

- You can sometimes find lists of things and relations online via full text search (aka Google) but that's because the internet is full of monkeys producing every possible combination of data that Google's indexing. Local search won't magically produce a list of items you want unless you made the list before hand.

- Wikipedia and many websites are full of graph data. I often go to wikipedia to find "the name of the song on the album that came out after the other album by this artist" where all I can remember is that the artist did a collaboration with this other artist. I find that by looking up the other artist, then following the links.

- Having data organized well makes it easy for software to consume - home assistants, importers, viewers, etc. You'd never have tax software generate a tax report by searching for transactions using full text search.

Also, while some of this data might be added by hand, I'd expect a lot of it to be added using automatic importers, perhaps reading from other databases of well-classified information.

Sunwet supports full text search too though.

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

- All JSON API, not even queries use a text DSL

- Optional [FDAP](https://github.com/andrewbaxter/openfdap) and [OIDC](https://github.com/andrewbaxter/fdap-oidc) user management

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
"sunwet/1/album" <- "sunwet/1/is" -> {
    id,
    "sunwet/1/name" -> first {
        name,
    }
}
```

So it starts with `"sunwet/1/album"`, moves _backwards_ (from object to subject) to find all nodes that are albums, and returns those as `id`. Then it does a subquery forward (subject to object) from each id, following name edges, and returning one name as `name`. The output will produce data like:

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
