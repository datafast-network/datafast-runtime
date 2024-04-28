# Datafast-Runtime

Document available at https://runtime.datafast.network

### Features

| Component | Args                 | Description                                                        |
|-----------|----------------------|--------------------------------------------------------------------|
| default   | `-F default`         | Default features including MongoDB and DeltaLake.                  |
| scylla    | `-F scylla`          | Feature for `Database` with dependency on ScyllaDB.                |
| mongo     | `-F mongo`           | Feature for `Database` with dependency on MongoDB.                 |
| pubsub    | `-F pubsub`          | Feature for `BlockSource` with dependency on Google Cloud Pub/Sub. |
| pubsub    | `-F pubsub_compress` | Feature for `BlockSource` with `lz4` compressed Pub/Sub support.   |
| deltalake | `-F deltalake`       | Feature for `BlockSource` with dependency on DeltaLake.            |

#### Examples

```shell
# Run app database mongodb and block source pubsub
cargo run -F mongo -F pubsub
```