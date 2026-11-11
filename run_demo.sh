cargo build -p locap2p-host
./apps/demo/gradlew -p apps/demo :app:run --args="--host-bin $PWD/target/debug/locap2p-host"
