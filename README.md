# Subgraph WASM Runtime

## Host Imports Functions

### Testing functions

- Export env for `test.wasm` path & `api_version`
```shell
export TEST_WASM_FILE=..path to test.wasm file
```

- Export env for API version (without env, default=0.0.5). Only 2 values accepted: 0.0.4 & 0.0.5
```
export RUNTIME_API_VERSION=0.0.4 # default being 0.0.5
```
