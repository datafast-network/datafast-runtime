# Subgraph WASM Runtime

## Design goal
The goal design is, the runtime must be very easy to use, very easy to pull a demo.
End-product should be an executable, can be run as CLI or separate Config file.

Example usage:
```shell
$ swr --manifest ~/my-subgraph-repo --subscribe nats://localhost:9000/blocks --store mystore://localhost:12345/namespace
```



## Architecture
```mermaid
sequenceDiagram
    participant MessageBus
    participant Subscriber
    participant SubgraphWasmHost
    participant Database
    participant Database

    Subscriber-->>MessageBus: binding connection
    Database-->>ExternalDatabase: binding connection
    Subscriber-->SubgraphWasmHost: kanal async channel
    SubgraphWasmHost-->>Database: request (StoreOperationMessage), synchronous
    Database-->>SubgraphWasmHost: response (StoreRequestResult)
    MessageBus->>Subscriber: Block/Tx/Event/Log

    rect rgb(191, 220, 255, .5)
    loop on message received
        note right of Subscriber: each class run in its own tokio-thread
        Subscriber->> SubgraphWasmHost: SubgraphOperationMessage(Job data)
        SubgraphWasmHost->>SubgraphWasmHost: processing triggers
        SubgraphWasmHost->>Database: StoreOperationMessage(Job)
    end
    end

    Database->>ExternalDatabase: database read/write
```

## Unit-Test

### Testing everything
1. Clone both this reop & [subgraph-testing](https://github.com/hardbed/subgraph-testing) repo and put them under the same directory
```shell
any-parent-dir $: git clone github.com/hardbed/subgraph-testing
any-parent-dir $: git clone github.com/hardbed/subgraph-wasm-runtime
```

2. Build the test suits first with [subgraph-testing](https://github.com/hardbed/subgraph-testing)
```shell
# install dependencies first if neccessary
subgraph-testing $: pnpm install
subgraph-testing $: pnpm build-test
```

3. In this repo, run test
```shell
subgraph-wasm-runtime $: RUST_LOG=info cargo test
```
