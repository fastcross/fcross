# Fcross

This is an academic proof-of-concept prototype of Fcross implemented in Cosmos ecosystem. 

## Dependencies

+ OS: Ubuntu 18.04 LTS or later.
+ Install `Rust` (1.81).
+ Install `Golang` (1.22).

+ Install `wasmd`.

```shell
git clone -b v0.28.0 https://github.com/CosmWasm/wasmd.git
make install
wasmd version
# 0.28.0
```

+ Install `relayer`.

```shell
git clone -b main https://github.com/sdgs72/relayer.git
make install
rly version
# v0.46.0
```

## Usage

Use the following command to start a designated number of blockchains and prepare relayers:

```shell
# or other chain numbers
./start.sh 4
```

Build the `go-client` and start to send cross-chain transactions through the client:

```shell
cd go-utils
go mod tidy
cd client && go build
./client
```

 After all transactions are finished, run the `go-analyser` to print the statistics:

```shell
cd go-utils/analyser && go build
./analyser
```

## Other evaluation

Use the following command to stop all running services:

```shell
./endAllGaia.sh
```

 Modify the `ContractName` in `start.sh` to deploy other two baselines or Fcross with the storage and execution optimizations.

```shell
# vanilla/avalon/fc-basic/fc-stor/fc-exec
contractName=fc-basic
```

