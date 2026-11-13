# IPC

## Overview

Applications run in a separate process and communicate with the framework host
over the host's standard input and output. The application starts the host
process and exchanges newline-delimited JSON (NDJSON) frames with it.

Each frame is one UTF-8 JSON object followed by a newline. The host flushes
after every frame, skips empty input lines, and rejects frames larger than
64 KiB (`MAX_FRAME_BYTES`).

## Startup

1. The application starts `locap2p-host`.
2. The host starts networking and writes a `ready` frame.
3. The application waits for `ready` before sending any frames.

```json
{"type":"ready","node_id":"12D3KooW..."}
```

## Application to host

| Frame | Fields | Meaning |
|---|---|---|
| `publish` | `payload` | Ask the host to create and publish an event |

```json
{"type":"publish","payload":{"x":1,"y":2}}
```

The payload is application-defined JSON. The host adds the event metadata:
author, sequence number, timestamp, and content-addressed ID.

## Host to application

| Frame | Fields | Meaning |
|---|---|---|
| `ready` | `node_id` | Host is ready; identifies the local node |
| `event` | `event` | A newly integrated local or remote event |
| `peer_joined` | `node_id` | A peer was discovered |
| `peer_left` | `node_id` | A peer expired from discovery |
| `error` | `message` | A publish or input frame failed |

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

- `publish` has no direct response. The host integrates the event into gossip,
  and the application receives it later as an `event` frame.
- Local and remote events use the same `event` frame. The application does not
  need a separate path for its own updates.
- Events are emitted only when newly integrated, so duplicates from push and
  pull are suppressed.
- Invalid input produces an `error` frame; the host then continues reading.
- Closing the application's write end of the host's stdin shuts the host down.

Peer discovery and the meaning of `peer_joined` and `peer_left` are described
in [discovery.md](discovery.md).
