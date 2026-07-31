#compdef sllend
# sllend zsh completion
# Install: fpath=(/path/to/scripts/sllend-completion $fpath) && compinit
# Or: source scripts/sllend-completion/sllend.zsh

_sllend_networks() { compadd testnet mainnet local }

_sllend_key_aliases() {
  if (( $+commands[jq] )) && [[ -f ~/.sllend/keystore.json ]]; then
    local aliases=( ${(f)"$(jq -r '.[].alias' ~/.sllend/keystore.json 2>/dev/null)"} )
    compadd -a aliases
  fi
}

_sllend_global_args=(
  '--network[Network to use]:network:_sllend_networks'
  '--contract[Override contract ID]:contract ID:'
  '--key[Keystore alias]:alias:_sllend_key_aliases'
  '--secret[Raw secret key (dev only)]:secret key:'
  '--account[Override sender address]:address:'
  '--json[Output raw JSON]'
  '--rpc-url[Override Soroban RPC URL]:url:'
  '(- *)--help[Show help]'
  '(- *)--version[Show version]'
)

_sllend() {
  local state

  _arguments -C \
    $_sllend_global_args \
    '1:command:->command' \
    '*::args:->args' && return

  case $state in
    command)
      local commands=(
        'deposit:Deposit collateral'
        'withdraw:Withdraw collateral'
        'borrow:Borrow assets'
        'repay:Repay debt'
        'liquidate:Liquidate a position'
        'get-position:Query user position'
        'get-pool:Query pool stats'
        'keys:Manage keystores'
        'config:Manage configuration'
        'interactive:Interactive REPL mode'
        'batch:Run a JSON command batch'
      )
      _describe 'command' commands ;;

    args)
      case $words[1] in
        deposit|withdraw|borrow|repay)
          _arguments \
            $_sllend_global_args \
            '--asset[Asset contract address]:address:' \
            '1:amount:' ;;
        liquidate)
          _arguments \
            $_sllend_global_args \
            '--collateral-asset[Collateral asset to seize]:address:' \
            '--debt-asset[Debt asset to repay]:address:' \
            '1:borrower address:' \
            '2:repay amount:' ;;
        get-position)
          _arguments $_sllend_global_args '1:address (optional):' ;;
        get-pool|interactive)
          _arguments $_sllend_global_args ;;
        keys)
          local subcmds=('add:Encrypt and store a key' 'list:List key aliases' 'remove:Remove a key')
          _describe 'subcommand' subcmds ;;
        config)
          local subcmds=(
            'show:Print current config'
            'set-network:Add or update a network entry'
            'set-contract:Set contract ID for a network'
            'set-default-network:Set the default network'
          )
          _describe 'subcommand' subcmds ;;
        batch)
          _arguments '1:batch file:_files -g "*.json"' ;;
      esac ;;
  esac
}

_sllend
