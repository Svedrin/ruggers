# Ruggers

Ruggers is an in-memory cache that supports multimaster replication and snapshots.

Writes are replicated to a maximum of 15 peers (meaning you can have 16 cluster
nodes in total) as soon as any one instance gets a write. Writes are async
and target nodes merge them seamlessly, if:

* they don't have the key locally at all, or
* they don't have a newer version of that key locally.

(Optimistic Locking: Let's hope that no-one meddled with our data in the meantime.)


## Protocol

Ruggers sits on a UDP port (default is `22422`) and waits for JSON data to arrive.
The easiest way to send some is using `ncat --udp 127.0.0.1 22422`.

Commands are:

* `{"Set":["key","value"]}`

    Sets `key` to `value`.

    Response: `"Ok"`

* `{"Get":"key"}`

    Get the current value of `key`, or `""` if unknown.

    Response: `{"Value":["key","value"]}`


## Replication

So you have multiple nodes, probably one on `192.168.122.78` and the other one
on `192.168.0.150`, and you want to replicate between them. Easy:

    root@192-168-0-150:/opt/ruggers# cargo run -- -r 192.168.122.78:22422 -n1

    root@192-168-122-78:/opt/ruggers# cargo run -- -r 192.168.0.150:22422 -n2

Now you can send some data to one of them (any one, there's no master):

    # ncat --udp 192.168.0.150 22422
    {"Set":["hallo1","lolol1"]}
    {"Set":["hallo2","lolol2"]}
    {"Set":["hallo3","lolol3"]}
    {"Set":["hallo4","lolol4"]}
    "Ok"
    "Ok"
    "Ok"
    "Ok"

And query it on the other:

    # ncat --udp 192.168.122.78 22422
    {"Get":"hallo1"}
    {"Get":"hallo2"}
    {"Get":"hallo3"}
    {"Get":"hallo4"}
    {"Value":["hallo1","lolol1"]}
    {"Value":["hallo2","lolol2"]}
    {"Value":["hallo3","lolol3"]}
    {"Value":["hallo4","lolol4"]}

Nodes will seamlessly merge the changes they get, if their local data isn't newer.
If it is, they'll `panic!` and you'll have to restart the whole cluster.
(I should probably improve that.)


## Snapshots

Insert a bunch of values:

    {"Set":["hallo1","lolol1"]}
    {"Set":["hallo2","lolol2"]}
    {"Set":["hallo3","lolol3"]}
    {"Set":["hallo4","lolol4"]}
    "Ok"
    "Ok"
    "Ok"
    "Ok"

Let's read them back:

    {"Get":"hallo1"}
    {"Get":"hallo2"}
    {"Get":"hallo3"}
    {"Get":"hallo4"}
    {"Value":["hallo1","lolol1"]}
    {"Value":["hallo2","lolol2"]}
    {"Value":["hallo3","lolol3"]}
    {"Value":["hallo4","lolol4"]}

Create a snapshot:

    {"SnapCreate":"testsnappen"}
    "Ok"

Modify some data:

    {"Set":["hallo3","omfg3"]}
    {"Set":["hallo4","omfg4"]}
    "Ok"
    "Ok"
    {"Get":"hallo1"}
    {"Get":"hallo2"}
    {"Get":"hallo3"}
    {"Get":"hallo4"}
    {"Value":["hallo1","lolol1"]}
    {"Value":["hallo2","lolol2"]}
    {"Value":["hallo3","omfg3"]}
    {"Value":["hallo4","omfg4"]}

Let's see what our snapshotted data is doing:

    {"SnapGet":["testsnappen","hallo3"]}
    {"Value":["hallo3","lolol3"]}
    {"SnapGet":["testsnappen","hallo4"]}
    {"Value":["hallo4","lolol4"]}

Hooray, it's still there! Let's delete the snapshot and see what happens:

    {"SnapDelete":"testsnappen"}
    "Ok"
    {"SnapGet":["testsnappen","hallo3"]}
    {"Value":["hallo3",""]}
    {"SnapGet":["testsnappen","hallo4"]}
    {"Value":["hallo4",""]}

Caveat: Snapshots are local to the node you take 'em on.


# Persistence

None, unfortunately. :(
