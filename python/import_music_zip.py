#!/bin/env python3
import typing
import typing_extensions
import dataclasses
from dataclasses import dataclass, field
import taglib
import zipfile
import argparse
import uuid
import json
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--zip", type=Path, help="Zip file extract to dir first")
parser.add_argument(
    "dir",
    type=Path,
    help="File containing albumset tracks and other associated files, where the sunwet.json will be written",
)
args = parser.parse_args()


# Notes
# disks are abstract in digital age, can have box set with multiple albums but single disks; albumset + album=disk
@dataclass(frozen=True)
class NodeId:
    id: str


@dataclass(frozen=True)
class NodeUpload:
    upload: str


@dataclass(frozen=True)
class NodeValue:
    value: typing.Any


Node = typing.Union[NodeId, NodeUpload, NodeValue]

PREFIX_SUNWET1 = "sunwet/1"

# Link to file node from metadata node representing file
PRED_FILE = f"{PREFIX_SUNWET1}/file"

# Human-known name for something
PRED_NAME = f"{PREFIX_SUNWET1}/name"

# A mangling of the human-known name that can be unambiguously sorted by a computer (ex: hiragana/katagana instead of kanji)
PRED_NAME_SORT = f"{PREFIX_SUNWET1}/name_sort"

# Link to artist
PRED_ARTIST = f"{PREFIX_SUNWET1}/artist"
PRED_VERSION = f"{PREFIX_SUNWET1}/version"
# Link to cover (file node)
PRED_COVER = f"{PREFIX_SUNWET1}/cover"
# Link to booklet (file node)
PRED_BOOKLET = f"{PREFIX_SUNWET1}/booklet"

# Collections: owner has first pointing to first element and element pointing to each element. Each element has next and index.
PRED_INDEX = f"{PREFIX_SUNWET1}/index"
PRED_ELEMENT = f"{PREFIX_SUNWET1}/element"

# Typing, can be chained to form hierarchy
PRED_IS = f"{PREFIX_SUNWET1}/is"


root_albumset_id = NodeId(id=f"{PREFIX_SUNWET1}/albumset")
root_album_id = NodeId(id=f"{PREFIX_SUNWET1}/album")
root_track_id = NodeId(id=f"{PREFIX_SUNWET1}/track")
root_artist_id = NodeId(id=f"{PREFIX_SUNWET1}/artist")


# Collect data
@dataclass
class Track:
    file: Path
    artist: typing.List[str] = field(default_factory=list)
    artist_sort: typing.List[str] = field(default_factory=list)
    name: typing.List[str] = field(default_factory=list)
    name_sort: typing.List[str] = field(default_factory=list)


@dataclass
class Album:
    tracks: typing.Dict[int, Track] = field(default_factory=dict)


@dataclass
class Albumset:
    covers: typing.List[Path] = field(default_factory=list)
    booklets: typing.List[Path] = field(default_factory=list)
    name: typing.List[str] = field(default_factory=list)
    name_sort: typing.List[str] = field(default_factory=list)
    artist: typing.List[str] = field(default_factory=list)
    artist_sort: typing.List[str] = field(default_factory=list)
    albums: typing.Dict[int, Album] = field(default_factory=dict)


albumset = Albumset()
if args.zip is not None:
    with zipfile.ZipFile(args.zip) as archive:
        archive.extractall(args.dir)

for f in args.dir.iterdir():
    if f.suffix in [".png", ".jpg", ".tiff", ".bmp"]:
        albumset.covers.append(f)
    elif f.suffix in [".epub", ".pdf", ".txt"]:
        albumset.booklets.append(f)
    elif f.suffix in [".ogg", ".mp3", ".wav", ".flac", ".aac", ".m4a"]:
        # Build tracks
        track_artist = []
        track_artist_sort = []
        track_name = []
        track_name_sort = []
        track_number = None
        disk_number = None
        for k, vs in taglib.File(f).tags.items():
            for v in vs:
                if k == "ALBUM":
                    albumset.name.append(v)
                elif k == "ALBUMSORT":
                    albumset.name_sort.append(v)
                elif k == "ARTIST":
                    track_artist.append(v)
                elif k == "ARTISTSORT":
                    track_artist_sort.append(v)
                elif k == "ALBUMARTIST":
                    albumset.artist.append(v)
                elif k == "ALBUMARTISTSORT":
                    albumset.artist_sort.append(v)
                elif k == "TITLE":
                    track_name.append(v)
                elif k == "TITLESORT":
                    track_name_sort.append(v)
                elif k == "TRACKNUMBER":
                    track_number = int(v.split("/")[0])
                elif k == "DISCNUMBER":
                    disk_number = int(k)
        album = albumset.albums.setdefault(
            disk_number or 1,
            Album(),
        )
        track = album.tracks.setdefault(
            track_number or 1,
            Track(
                file=f,
            ),
        )
        track.artist = track_artist
        track.artist_sort = track_artist_sort
        track.name = track_name
        track.name_sort = track_name_sort


# Build triples...
@dataclass(frozen=True)
class Triple:
    subject: Node
    predicate: str
    object: Node


def triple(sub: Node, pred: str, obj: Node) -> Triple:
    return Triple(
        subject=sub,
        predicate=pred,
        object=obj,
    )


def node_upload(path) -> Node:
    return NodeUpload(upload=str(path.relative_to(args.dir)))


def node_id() -> Node:
    return NodeId(id=str(uuid.uuid4()).lower())


def node_value(value) -> Node:
    return NodeValue(value=value)


triples: typing.Set[Triple] = set()
artists: typing.Dict[str, Node] = {}


def build_artist(name: str, name_sort: str) -> Node:
    artist_id = artists.get(name)
    if artist_id is None:
        artist_id = node_id()
    artists[v] = artist_id
    triples.add(triple(artist_id, PRED_IS, root_artist_id))
    triples.add(triple(artist_id, PRED_NAME, node_value(name)))
    triples.add(triple(artist_id, PRED_NAME_SORT, node_value(name_sort)))
    return artist_id


# Albumset
albumset_id = node_id()
triples.add(triple(albumset_id, PRED_IS, root_albumset_id))
for v in albumset.covers:
    triples.add(triple(albumset_id, PRED_COVER, node_upload(v)))
for v in albumset.booklets:
    triples.add(triple(albumset_id, PRED_BOOKLET, node_upload(v)))
for v in set(albumset.name):
    triples.add(triple(albumset_id, PRED_NAME, node_value(v)))
for v in set(albumset.name_sort):
    triples.add(triple(albumset_id, PRED_NAME_SORT, node_value(v)))
albumset_artist = set(zip(albumset.artist, albumset.artist_sort or albumset.artist))
if albumset_artist is None:
    albumset_artist = set([("Various Artists", "Various Artists")])
for v, v_sort in albumset_artist:
    triples.add(triple(albumset_id, PRED_ARTIST, build_artist(v, v_sort)))

# Albums
for real_index, (index, album) in enumerate(sorted(albumset.albums.items())):
    album_id = node_id()
    triples.add(triple(albumset_id, PRED_ELEMENT, album_id))
    triples.add(triple(album_id, PRED_INDEX, node_value(index)))
    triples.add(triple(album_id, PRED_IS, root_album_id))

    if len(albumset.albums) == 1:
        triples.add(
            triple(
                album_id,
                PRED_NAME,
                node_value(albumset.name[0]),
            )
        )
        for album_name_sort in albumset.name_sort:
            triples.add(
                triple(
                    album_id,
                    PRED_NAME_SORT,
                    node_value(album_name_sort),
                )
            )
            break
    else:
        triples.add(
            triple(
                album_id,
                PRED_NAME,
                node_value("{} (Disk {})".format(albumset.name[0], index)),
            )
        )
        for album_name_sort in albumset.name_sort:
            triples.add(
                triple(
                    album_id,
                    PRED_NAME_SORT,
                    node_value("{} (Disk {})".format(album_name_sort, index)),
                )
            )
            break
    for v, v_sort in albumset_artist:
        triples.add(triple(album_id, PRED_ARTIST, build_artist(v, v_sort)))

    # Tracks
    for real_track_index, (track_index, track) in enumerate(
        sorted(album.tracks.items())
    ):
        track_id = node_id()
        triples.add(triple(album_id, PRED_ELEMENT, track_id))
        triples.add(triple(track_id, PRED_INDEX, node_value(track_index)))
        triples.add(triple(track_id, PRED_IS, root_track_id))

        triples.add(triple(track_id, PRED_FILE, node_upload(track.file)))
        for v in set(track.name):
            triples.add(triple(track_id, PRED_NAME, node_value(v)))
        for v in set(track.name_sort):
            triples.add(triple(track_id, PRED_NAME_SORT, node_value(v)))
        for v, v_sort in set(zip(track.artist, track.artist_sort or track.artist)):
            triples.add(triple(track_id, PRED_ARTIST, build_artist(v, v_sort)))

open(args.dir / "sunwet.json", "w").write(
    json.dumps(
        {"add": list(map(lambda x: dataclasses.asdict(x), list(triples)))}, indent=4
    )
)
