# IPC

## Overview

Applications run in a separate process and communicate with the framework host
over the host's standard input and output. The application starts the host
process and exchanges newline-delimited JSON (NDJSON) frames with it.

Each frame is one UTF-8 JSON object followed by a newline. The host flushes
after every frame, skips empty input lines, and rejects frames larger than
64 KiB.

## Startup

1. The application loads its configuration file.
2. The application starts the host process.
3. The application sends a configure frame as the first frame.
4. The host loads the identity and starts the network, bootstrap discovery,
   relay connectivity, and gossip.
5. The host writes a ready frame.
6. The application waits for the ready frame before sending publish frames.

`ready` means the configuration was accepted and local networking started. It
does not mean that the bootstrap node has been reached or that peers have been
discovered.

```json
{"type":"ready","node_id":"12D3KooW..."}
```

## Application to host

| Frame | Fields | Meaning |
|---|---|---|
| configure | config | Supply the application configuration; must be the first frame |
| publish | payload | Ask the host to create and publish an event |

Configure frame:

```json
{"type":"configure","config":{"schema_version":1,"application":{...}}}
```

The full configuration schema is the same one described in
[configuration.md](configuration.md).

```json
{"type":"publish","payload":{"x":1,"y":2}}
```

The payload is application-defined JSON. The host adds the event metadata:
author, sequence number, timestamp, and content-addressed identifier.

## Host to application

| Frame | Fields | Meaning |
|---|---|---|
| ready | node_id | Host is ready; identifies the local node |
| event | event | A newly integrated local or remote event |
| peer_joined | node_id | A peer was discovered |
| peer_left | node_id | A peer expired from discovery |
| error | message | A publish or input frame failed |

Event frame:

```json
{
  "type": "event",
  "event": {
    "id": "7f...",
    "author": "12D3KooW...",
    "seq": 1,
    "timestamp_nanos": 1760000000000000000,
    "payload": {"x":1,"y":2}
  }
}
```

## Delivery semantics

- Publishing has no direct response. The host integrates the event into gossip,
  and the application receives it later as an event frame.
- Local and remote events use the same event frame. The application does not
  need a separate path for its own updates.
- Events are emitted only when newly integrated, so duplicates from push and
  pull are suppressed.
- A configuration that cannot start the network, or a non-configure first frame,
  produces an error frame and the host exits.
- Invalid input after configuration produces an error frame; the host then
  continues reading.
- A second configure frame is rejected because the host is already configured.
- Closing the application's write end of the host's standard input shuts the
  host down.

Peer discovery and the meaning of the peer-joined and peer-left frames are
described in [discovery.md](discovery.md). Direct and relayed connectivity is
described in [connectivity.md](connectivity.md).
