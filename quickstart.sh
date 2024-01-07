export AWS_ENDPOINT=https://sgp1.digitaloceanspaces.com
export AWS_REGION=us-east-1
export AWS_S3_ALLOW_UNSAFE_RENAME=true
export AWS_SECRET_ACCESS_KEY=IK/yRuxKoNiriRK3MAPG3TSkTp89/ju7jHsUczGnzF0
export AWS_ACCESS_KEY_ID=DO00RZ72K99QNRXE4FWN
export AWS_ALLOW_HTTP=true
export AWS_FORCE_PATH_STYLE=true
export TABLE_PATH=s3://dfr-ethereum/
export RUST_LOG=info,deltalake=off
export CONFIG=quickstart_config.toml

docker compose up -d
cargo run --release
