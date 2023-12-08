# Datafast-Runtime

## Design goal
The goal design is, the runtime must be very easy to use, very easy to pull a demo.
End-product should be an executable, can be run as CLI or separate Config file.

Example usage:
```shell
$ dfr --manifest ~/my-subgraph-repo --database mystore://localhost:12345/namespace
```



## Architecture
```mermaid
sequenceDiagram
    participant RemoteBlockStore;
    participant BlockSource;
    participant DataFilter;
    participant Main;
    participant BlockInspector;
    participant Subgraph;
    participant Database;

    loop
        BlockSource->>+RemoteBlockStore: query blocks
        RemoteBlockStore-->>BlockSource: block-batch
        BlockSource->>BlockSource: serialize

        BlockSource->>DataFilter: block-data-full
        DataFilter->>DataFilter: sorting & filtering data
        DataFilter->>Main: per-block-wrapped data batch

        loop: loop through each block
            Main->>BlockInspector: validate block-pointer
            BlockInspector->>Subgraph: valid block
            Subgraph->>Subgraph: process & save data point to memory
        end

        Subgraph->>Database: commit
        Main->>Database: remove outdated snapshots
        Main->>Database: clean up data-history if needed & flush cache
    end
```

## Component usage

### ManifestLoader

- Accept local subgraph dir for constructor
```rust
let loader = ManifestLoader::new("fs://vutran/works/subgraph-testing/packages/v0_0_5").await.unwrap()
```

## Unit-Test

### Testing everything
1. Clone both this reop & [subgraph-testing](https://github.com/hardbed/subgraph-testing) repo and put them under the same directory
```shell
any-parent-dir $: git clone github.com/hardbed/subgraph-testing
any-parent-dir $: git clone github.com/hardbed/subgraph-wasm-runtime
```

2. Build the test suites first with [subgraph-testing](https://github.com/hardbed/subgraph-testing)
```shell
# install dependencies first if neccessary
subgraph-testing $: pnpm install
subgraph-testing $: pnpm build
```

3. In this repo, run test
```shell
subgraph-wasm-runtime $: RUST_LOG=info cargo test
```
