## My local nostr gRPC Server
[![License](https://img.shields.io/badge/License-BSD_3--Clause-blue.svg)](LICENSE)

# gRPC Extensions for nostr-rs-relay

gRPC authz server for [nostr-rs-rely](https://github.com/scsibug/nostr-rs-relay). Admits events based on whether the author has been allowed by the admit. Events that are referenced by an admitted event will be fetch from the `default-relays` set in the config or from relays set in the `r` tag.

# Motivation

The goal of this is to allow people to run there own lightweight home relay to back up notes that there care about.  Currently, many are running home relays to backup their own notes but by simply backing up your notes the context the note exists in is lost, what good is backing up a reply without the comment it is a reply to?

## Managing Users

### Via Nostr

The admin(s) can update accounts by publishing an `kind` 4242 event with an allow tag where index 0 is "allow" followed by the list of hex pubkeys, and a "deny" tag of the same format.
 
For now this is not in a NIP if there is interest it can be more formalized.

Events can be published using this branch of nostr tools or implementing the event format in other tools.

https://github.com/thesimplekid/nostr-tool/tree/manage_relay_users

```json
{
  "id": <32-bytes lowercase hex-encoded sha256 of the the serialized event data>,
  "pubkey": <pubkey of the relay admin>,
  "created_at": <unix timestamp in seconds>,
  "kind": 4242,
  "tags": [
    ["allow", <32-bytes hex of a pubkey>,  <32-bytes hex of a pubkey>, ...],
    ["deny", <32-bytes hex of a pubkey>, <32-bytes hex of a pubkey>, ...],
    ...
  ],
  "content": "", 
  ...
}

```

### HTTP API
The users can be updated by sending a http `POST` to the  `/update` endpoint with a json body with the following format.

```json
{
    "allow":, [<32-bytes hex of a pubkey>,  <32-bytes hex of a pubkey>, ...],
    "deny": [<32-bytes hex of a pubkey>, <32-bytes hex of a pubkey>, ...],
}
```

There is also a `GET` endpoint with at `/users` that will return json of the same format with allowed and denied users.


If the relay has nip42 enabled it will use the authenticated pubkey if not the author pubkey of the note will be used. 


## License 
Code is under the [BSD 3-Clause License](LICENSE-BSD-3)

## Contact

I can be contacted for comments or questions on nostr at _@thesimplekid.com (npub1qjgcmlpkeyl8mdkvp4s0xls4ytcux6my606tgfx9xttut907h0zs76lgjw) or via email tsk@thesimplekid.com.
