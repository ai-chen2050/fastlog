# Aos Proposer

### Run proposer

```shell
cargo build --features nitro-enclaves --release

./target/release/proposer-runer -i postgres://postgres:hetu@0.0.0.0:5432/proposer_db

./target/release/proposer-runer  -c ./docs/template/config-proposer.yaml
```