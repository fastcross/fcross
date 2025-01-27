#!/bin/bash
set -x
set -e

if [ "$#" -ne 7 ]; then
  echo "Error: Expected 7 arguments, but got $#."
  exit 1
fi

KEYRING=--keyring-backend="test"

BINARY=$1
CHAINID=$2
CHAINHOME=$3
RPCPORT=$4
P2PPORT=$5
PROFPORT=$6
GRPCPORT=$7

echo "Creating $BINARY instance: home=$CHAINHOME | chain-id=$CHAINID | p2p=:$P2PPORT | rpc=:$RPCPORT | profiling=:$PROFPORT | grpc=:$GRPCPORT"

# Add dir for chain, exit if error
mkdir $CHAINHOME

# Build genesis file incl account for passed address
coins="100000000000stake,100000000000samoleans"
delegate="100000000000stake"

$BINARY --home $CHAINHOME --chain-id $CHAINID init $CHAINID
# Derive a new private key and encrypt to disk: wasmd keys add <name> [flags]
$BINARY --home $CHAINHOME keys add validator $KEYRING --output json > $CHAINHOME/validator_seed.json 2>&1
$BINARY --home $CHAINHOME keys add user $KEYRING --output json > $CHAINHOME/key_seed.json 2>&1
$BINARY --home $CHAINHOME keys add testkey $KEYRING --output json > $CHAINHOME/testkey_seed.json 2>&1
# Add a genesis account to genesis.json: wasmd add-genesis-account [address_or_key_name] [coin][,[coin]] [flags]
$BINARY --home $CHAINHOME add-genesis-account $($BINARY --home $CHAINHOME keys $KEYRING show user -a) $coins
$BINARY --home $CHAINHOME add-genesis-account $($BINARY --home $CHAINHOME keys $KEYRING show validator -a) $coins 
$BINARY --home $CHAINHOME add-genesis-account $($BINARY --home $CHAINHOME keys $KEYRING show testkey -a) $coins 
# Generate a genesis transaction that creates a validator with a self-delegation: wasmd gentx my-key-name 1000000stake ...
$BINARY --home $CHAINHOME gentx validator $delegate $KEYRING --chain-id $CHAINID
# Collect genesis txs and output a genesis.json file
$BINARY --home $CHAINHOME collect-gentxs

# Set proper defaults and change ports (use a different sed for Mac or Linux)
echo "Change settings in config.toml file..."
sed -i 's#"tcp://127.0.0.1:26657"#"tcp://0.0.0.0:'"$RPCPORT"'"#g' $CHAINHOME/config/config.toml
sed -i 's#"tcp://0.0.0.0:26656"#"tcp://0.0.0.0:'"$P2PPORT"'"#g' $CHAINHOME/config/config.toml
sed -i 's#"localhost:6060"#"localhost:'"$PROFPORT"'"#g' $CHAINHOME/config/config.toml
sed -i 's/timeout_commit = "5s"/timeout_commit = "1s"/g' $CHAINHOME/config/config.toml
sed -i 's/timeout_propose = "3s"/timeout_propose = "1s"/g' $CHAINHOME/config/config.toml
sed -i 's/index_all_keys = false/index_all_keys = true/g' $CHAINHOME/config/config.toml
# sed -i '' 's#index-events = \[\]#index-events = \["message.action","send_packet.packet_src_channel","send_packet.packet_sequence"\]#g' $CHAINHOME/config/app.toml

# Start the gaia
$BINARY --home $CHAINHOME start --pruning=nothing --grpc-web.enable=false --grpc.address="0.0.0.0:$GRPCPORT" > $(dirname $CHAINHOME)/$CHAINID.log 2>&1 &

echo "$CHAINID initialized. Watch file $(dirname $CHAINHOME)/$CHAINID.log to see its execution."