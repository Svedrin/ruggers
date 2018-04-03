# Ruggers

Ruggers is an in-memory cache that supports multimaster replication.

Writes are replicated to a maximum of 15 peers (meaning you can have 16 cluster
nodes in total) as soon as any one instance gets a write. Writes are async
and target nodes merge them seamlessly, if:

* they don't have the key locally at all, or
* they don't have a newer version of that key locally.

(Optimistic Locking: Let's hope that no-one meddled with our data in the meantime.)

## Protocol

Ruggers sits on a UDP port (default is `22422`) and waits for JSON data to arrive.
Commands are:

* `{"Set":["key","value"]}`

    Sets `key` to `value`.

    Response: `"Ok"`

* `{"Get":"key"}`

    Get the current value of `key`, or `""` if unknown.

    Response: `{"Value":["key","value"]}`
