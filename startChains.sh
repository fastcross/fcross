#!/bin/bash
set -x
set -e

deployTestSmartContract() {
  contractName=mycontract
  # these msg should be included in "" when called to prevent being split
  initMsg='{"admins": ["wasm1rcweqkrqswyaudxy5v7gsa5mygyfdhtsvhk5r2"], "donation_denom": "mytoken"}'
  queryMsg='{ "admins_list": {} }'

  smartContractDir=$SCRIPTDIR/contracts/$contractName
  cd $smartContractDir
  RUSTFLAGS='-C link-arg=-s' cargo +1.69.0 build --target wasm32-unknown-unknown --release --lib
  cd -

  # deploy smart contract per chain
  for((i=0;i<$1;i++)); do
    # contract
    homeFlag="--home $WASMD_DATA/ibc-$i"
    rpcFlag="--node http://127.0.0.1:$((26550+i))"
    gasFlag='--gas-prices 0.025stake --gas 20000000 --gas-adjustment 1.1'
    wasmBinary="$smartContractDir/target/wasm32-unknown-unknown/release/$contractName.wasm"
    wasmd $homeFlag tx wasm store $wasmBinary $rpcFlag --from user --chain-id ibc-$i --gas-prices "0.025stake" --gas "20000000" --broadcast-mode block -y --keyring-backend test
    codeId=$(wasmd $homeFlag query wasm list-code $rpcFlag --output json | jq -r ".code_infos[-1] | .code_id")
    wasmd $homeFlag tx wasm instantiate $codeId "$initMsg" $rpcFlag --from user --chain-id ibc-$i $gasFlag --broadcast-mode block -y --keyring-backend test --label "hello" --no-admin
    # query the contract instance address
    contractAddr=$(wasmd $homeFlag query wasm list-contract-by-code $codeId $rpcFlag --output json | jq -r '.contracts[-1]')
    wasmd $homeFlag query wasm contract-state smart $contractAddr "$queryMsg" $rpcFlag
  done
}

deployMf1() {
  contractName=mf1
  initMsg='{"chain_id": 0, "original_value": 20}'
  execMsg='{"execute_tx":{"fcross_tx":{"tx_id":1,"operation":{"debit_balance":{"amount":7}}}}}'
  queryMsg='{ "all_futures": {} }'

  smartContractDir=$SCRIPTDIR/contracts/$contractName
  cd $smartContractDir
  RUSTFLAGS='-C link-arg=-s' cargo +1.69.0 build --target wasm32-unknown-unknown --release --lib
  cd -

  # deploy smart contract per chain
  for((i=0;i<$1;i++)); do
    # contract
    homeFlag="--home $WASMD_DATA/ibc-$i"
    rpcFlag="--node http://127.0.0.1:$((26550+i))"
    gasFlag='--gas-prices 0.025stake --gas 20000000 --gas-adjustment 1.1'
    wasmBinary="$smartContractDir/target/wasm32-unknown-unknown/release/$contractName.wasm"
    wasmd $homeFlag tx wasm store $wasmBinary $rpcFlag --from user --chain-id ibc-$i --gas-prices "0.025stake" --gas "20000000" --broadcast-mode block -y --keyring-backend test
    codeId=$(wasmd $homeFlag query wasm list-code $rpcFlag --output json | jq -r ".code_infos[-1] | .code_id")
    wasmd $homeFlag tx wasm instantiate $codeId "$initMsg" $rpcFlag --from user --chain-id ibc-$i $gasFlag --broadcast-mode block -y --keyring-backend test --label "hello" --no-admin
    # query the contract instance address
    contractAddr=$(wasmd $homeFlag query wasm list-contract-by-code $codeId $rpcFlag --output json | jq -r '.contracts[-1]')
    wasmd $homeFlag tx wasm execute $contractAddr "$execMsg" $gasFlag $rpcFlag --chain-id ibc-$i --from user --broadcast-mode block -y --keyring-backend test
    wasmd $homeFlag query wasm contract-state smart $contractAddr "$queryMsg" $rpcFlag
  done
}

deployMfAndCoordinator() {
  logicChainNum=$(($1 - 1))
  if (( $logicChainNum <= 0 )); then
    echo "should have at least 1 logic chain" >&2
    exit 1
  fi

  # contracts dir
  # vanilla/avalon/fc-basic/fc-stor/fc-exec
  contractName=fc-basic
  coordinatorName=coordinator1
  if [[ "${contractName}" == "avalon" ]]; then
    coordinatorName=coordinator2
  fi
  

  # compile contracts
  cd $SCRIPTDIR/contracts/$contractName
  RUSTFLAGS='-C link-arg=-s' cargo +1.69.0 build --target wasm32-unknown-unknown --release --lib
  cd -
  cd $SCRIPTDIR/contracts/$coordinatorName
  RUSTFLAGS='-C link-arg=-s' CHAIN_NUM=$1 cargo +1.69.0 build --target wasm32-unknown-unknown --release --lib
  cd -

  # deploy smart contract per chain, chain-0 as coordinator
  # collect ibc ports
  ibcPorts=()
  for((i=0;i<$1;i++)); do
    homeFlag="--home $WASMD_DATA/ibc-$i"
    rpcFlag="--node http://127.0.0.1:$((26550+i))"
    gasFlag='--gas-prices 0.025stake --gas 40000000 --gas-adjustment 1.1'
    
    if (( i == 0 )); then
      initMsg="{\"chain_num\":$logicChainNum}"
      execMsg='{"add_vote":{"vote":{"tx_id":1,"chain_id":0,"success":true}}}'
      queryMsg='{"opening_votes":{}}'
      wasmBinary="$SCRIPTDIR/contracts/$coordinatorName/target/wasm32-unknown-unknown/release/$coordinatorName.wasm"
    else
      initMsg="{\"chain_id\": $i, \"original_value\": 1000}"
      execMsg='{"execute_tx":{"fcross_tx":{"tx_id":1,"operation":{"debit_balance":{"amount":7}}}}}'
      queryMsg='{"multifuture":{"tx_id":1}}'
      wasmBinary="$SCRIPTDIR/contracts/$contractName/target/wasm32-unknown-unknown/release/$contractName.wasm"
    fi
    
    wasmd $homeFlag tx wasm store $wasmBinary $rpcFlag --from user --chain-id ibc-$i --gas-prices "0.025stake" --gas "20000000" --broadcast-mode block -y --keyring-backend test
    codeId=$(wasmd $homeFlag query wasm list-code $rpcFlag --output json | jq -r ".code_infos[-1] | .code_id")
    wasmd $homeFlag tx wasm instantiate $codeId "$initMsg" $rpcFlag --from user --chain-id ibc-$i $gasFlag --broadcast-mode block -y --keyring-backend test --label "hello" --no-admin
    # query the contract instance address
    contractAddr=$(wasmd $homeFlag query wasm list-contract-by-code $codeId $rpcFlag --output json | jq -r '.contracts[-1]')
    # query its ibc port
    contractIbcPort=$(wasmd $homeFlag query wasm contract $contractAddr $rpcFlag --output --json | jq -r '.contract_info | .ibc_port_id')
    ibcPorts+=("$contractIbcPort")
    # wasmd $homeFlag tx wasm execute $contractAddr "$execMsg" $gasFlag $rpcFlag --chain-id ibc-$i --from user --broadcast-mode block -y --keyring-backend test
    # wasmd $homeFlag query wasm contract-state smart $contractAddr "$queryMsg" $rpcFlag
  done
}

# input check
chainNum=$1
if [ -z $1 ]; then
    echo "Need Number of nodes for deploying..." >&2
    exit 1
fi

SCRIPTDIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
WASMD_DATA="${SCRIPTDIR}/data"
RELAYER_HOME="${SCRIPTDIR}/rly_data"
RELAYER_LOGS="${SCRIPTDIR}/relayer_logs"
# RELAYER_HOME="$HOME/.relayer"

# preparation
if ! [ -x "$(which wasmd)" ]; then
  echo "wasmd unavailable" >&2
  exit 1
fi
if [[ ! -x "$(which jq)" ]]; then
  echo "jq (a tool for parsing json in the command line) unavailable" >&2
  exit 1
fi

# Delete wasmd_data/relayer_folders/relayer_logs, clean previous processes
rm -rf $WASMD_DATA
rm -rf $RELAYER_HOME
rm -rf $RELAYER_LOGS
killall wasmd || true
killall rly || true

# start chains
mkdir $WASMD_DATA
echo "starting $chainNum chains..."
for ((i=0;i<chainNum;i++)); do
    chainId="ibc-$i"
    ./oneChain.sh wasmd $chainId $WASMD_DATA/$chainId $((26550+i)) $((26660+i)) $((6060+i)) $((9090+i))
done
sleep 1 # wait for chain rpc service to work


# deploy contracts
echo "deploying smart contract..."
deployMfAndCoordinator $chainNum
echo "deploy smart contract done!"


# config and start rly
echo "Starting all relayers in the background...."
mkdir $RELAYER_HOME
mkdir $RELAYER_LOGS
rly --home $RELAYER_HOME config init
for ((i=0;i<chainNum;i++)); do
    rly --home $RELAYER_HOME chains add -f configs/wasmd/chains/ibc-$i.json
    seed=$(jq -r '.mnemonic' $WASMD_DATA/ibc-$i/testkey_seed.json)
    echo "Key $(rly --home $RELAYER_HOME keys restore ibc-$i testkey "$seed") imported from ibc-$i to relayer..."
    # establish path of ibc-i with ibc-0 (coordinator)
    if (( i != 0 )); then
      # create connection between chains
      rly --home $RELAYER_HOME paths new ibc-0 ibc-$i mypath0-$i
      # create channel between contracts
      rly --home $RELAYER_HOME tx link mypath0-$i --src-port ${ibcPorts[0]} --dst-port ${ibcPorts[i]} --order unordered --version v1
      rly --home $RELAYER_HOME start mypath0-$i --debug-addr localhost:750$i > $RELAYER_LOGS/mypath0-$i.log 2>&1 &
    fi
    # delete user to rename it to ibc-$i, --home and --keyring-backend flags are necessary for wasmd
    # wasmd --home $WASMD_DATA/ibc-$i keys delete user -y --keyring-backend="test" || true
    # cat $WASMD_DATA/ibc-$i/key_seed.json | jq .mnemonic -r | wasmd --home $WASMD_DATA/ibc-$i keys add ibc-$i --recover --keyring-backend="test"
done
echo "start rly done!"





