# Datafast-Runtime

Document available at [https://github.com/vutran1710](https://runtime.datafast.network/)

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
