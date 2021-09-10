#/usr/bin/env bash

_jq_installed() {
  if ! command -v jq &> /dev/null; then
    exit
  fi
}

_available_profiles() {
  profile_file="$1"
  cat $profile_file |jq --raw-output 'keys |join("\n")'
}

_config_file() {
  config_file="fhttp-config.json"
  index=0
  for i in ${COMP_WORDS[@]}; do
    if [ "$i" == "-f" ] || [ "$i" == "--profile-file" ]; then
      next_index=$((index + 1))

      tmp="${COMP_WORDS[next_index]}"
      if [ "${tmp}" != "" ]; then
        config_file="$tmp"
        break
      else
        break
      fi
    fi
    index=$((index + 1))
  done

  echo "${config_file}"
}

_fhttp_completions() {
  _jq_installed
  config_file=$(_config_file)

  current_word="${COMP_WORDS[$COMP_CWORD - 1]}"
  if [ "$current_word" == "-p" ] || [ "$current_word" == "--profile" ]; then
    for profile in $(_available_profiles "$config_file"); do
      COMPREPLY+=("$profile")
    done
  fi
}

complete -o dirnames -X '*.http' -F _fhttp_completions fhttp
