#!/usr/bin/env bash
# sllend bash completion
# Install: source scripts/sllend-completion/sllend.bash
# Or add to ~/.bashrc: [ -f /path/to/sllend.bash ] && source /path/to/sllend.bash

_sllend_global_opts="--network --contract --key --secret --account --json --rpc-url --help --version"
_sllend_networks="testnet mainnet local"

_sllend_completions() {
  local cur prev words cword
  _init_completion || return

  local commands="deposit withdraw borrow repay liquidate get-position get-pool keys config interactive batch"

  case "$prev" in
    --network)  COMPREPLY=( $(compgen -W "$_sllend_networks" -- "$cur") ); return ;;
    --key)
      # List aliases from keystore if jq available
      if command -v jq &>/dev/null && [[ -f ~/.sllend/keystore.json ]]; then
        local aliases
        aliases="$(jq -r '.[].alias' ~/.sllend/keystore.json 2>/dev/null)"
        COMPREPLY=( $(compgen -W "$aliases" -- "$cur") )
      fi
      return ;;
    --contract|--account|--secret|--rpc-url) return ;;
    batch) _filedir '*.json'; return ;;
  esac

  # Find the subcommand position
  local subcmd=""
  for word in "${words[@]:1}"; do
    case "$word" in
      deposit|withdraw|borrow|repay|liquidate|get-position|get-pool|keys|config|interactive|batch)
        subcmd="$word"; break ;;
    esac
  done

  if [[ -z "$subcmd" ]]; then
    if [[ "$cur" == -* ]]; then
      COMPREPLY=( $(compgen -W "$_sllend_global_opts" -- "$cur") )
    else
      COMPREPLY=( $(compgen -W "$commands" -- "$cur") )
    fi
    return
  fi

  case "$subcmd" in
    deposit|withdraw|borrow|repay)
      COMPREPLY=( $(compgen -W "--asset $_sllend_global_opts" -- "$cur") ) ;;
    liquidate)
      COMPREPLY=( $(compgen -W "--collateral-asset --debt-asset $_sllend_global_opts" -- "$cur") ) ;;
    get-position|get-pool|interactive)
      COMPREPLY=( $(compgen -W "$_sllend_global_opts" -- "$cur") ) ;;
    keys)
      COMPREPLY=( $(compgen -W "add list remove" -- "$cur") ) ;;
    config)
      COMPREPLY=( $(compgen -W "show set-network set-contract set-default-network" -- "$cur") ) ;;
    batch)
      _filedir '*.json' ;;
  esac
}

complete -F _sllend_completions sllend
