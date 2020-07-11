complete -c tunelo -n "__fish_use_subcommand" -s c -l config
complete -c tunelo -n "__fish_use_subcommand" -s h -l help -d 'Prints help information'
complete -c tunelo -n "__fish_use_subcommand" -s V -l version -d 'Prints version information'
complete -c tunelo -n "__fish_use_subcommand" -f -a "version" -d 'Shows current version'
complete -c tunelo -n "__fish_use_subcommand" -f -a "completions" -d 'Shows shell completions'
complete -c tunelo -n "__fish_use_subcommand" -f -a "multi-proxy" -d 'Starts multiple proxy server'
complete -c tunelo -n "__fish_use_subcommand" -f -a "proxy-chain" -d 'Runs as proxy chain server'
complete -c tunelo -n "__fish_use_subcommand" -f -a "proxy-checker" -d 'Runs as proxy checker'
complete -c tunelo -n "__fish_use_subcommand" -f -a "socks-server" -d 'Runs as SOCKS proxy server'
complete -c tunelo -n "__fish_use_subcommand" -f -a "http-server" -d 'Runs as HTTP proxy server'
complete -c tunelo -n "__fish_use_subcommand" -f -a "help" -d 'Prints this message or the help of the given subcommand(s)'
complete -c tunelo -n "__fish_seen_subcommand_from version" -s h -l help -d 'Prints help information'
complete -c tunelo -n "__fish_seen_subcommand_from version" -s V -l version -d 'Prints version information'
complete -c tunelo -n "__fish_seen_subcommand_from completions" -s h -l help -d 'Prints help information'
complete -c tunelo -n "__fish_seen_subcommand_from completions" -s V -l version -d 'Prints version information'
complete -c tunelo -n "__fish_seen_subcommand_from multi-proxy" -s c -l config
complete -c tunelo -n "__fish_seen_subcommand_from multi-proxy" -s h -l help -d 'Prints help information'
complete -c tunelo -n "__fish_seen_subcommand_from multi-proxy" -s V -l version -d 'Prints version information'
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -s c -l config
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -l socks-ip
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -l socks-port
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -l http-ip
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -l http-port
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -l proxy-chain-file
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -l proxy-chain
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -l disable-socks4a
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -l disable-socks5
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -l disable-http
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -s h -l help -d 'Prints help information'
complete -c tunelo -n "__fish_seen_subcommand_from proxy-chain" -s V -l version -d 'Prints version information'
complete -c tunelo -n "__fish_seen_subcommand_from proxy-checker" -s c -l config
complete -c tunelo -n "__fish_seen_subcommand_from proxy-checker" -s s -l proxy-servers -d 'Proxy server list'
complete -c tunelo -n "__fish_seen_subcommand_from proxy-checker" -s f -l file -d 'Proxy server list file'
complete -c tunelo -n "__fish_seen_subcommand_from proxy-checker" -s o -l output-file
complete -c tunelo -n "__fish_seen_subcommand_from proxy-checker" -s p -l probers -d 'Proxy probers'
complete -c tunelo -n "__fish_seen_subcommand_from proxy-checker" -l max-timeout-per-probe -d 'Max timeout per probe in millisecond'
complete -c tunelo -n "__fish_seen_subcommand_from proxy-checker" -s h -l help -d 'Prints help information'
complete -c tunelo -n "__fish_seen_subcommand_from proxy-checker" -s V -l version -d 'Prints version information'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -s c -l config
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -l ip -d 'IP address to listen'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -l port -d 'Port number to listen'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -l disable-socks4a -d 'Disable SOCKS4a support'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -l disable-socks5 -d 'Disable SOCKS5 support'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -l enable-tcp-connect -d 'Enable "TCP Connect" support'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -l enable-tcp-bind -d 'Enable "TCP Bind" support'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -l enable-udp-associate -d 'Enable "UDP Associate" support'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -l udp-ports -d 'UDP ports to provide UDP associate service'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -l connection-timeout -d 'Connection timeout'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -s h -l help -d 'Prints help information'
complete -c tunelo -n "__fish_seen_subcommand_from socks-server" -s V -l version -d 'Prints version information'
complete -c tunelo -n "__fish_seen_subcommand_from http-server" -s c -l config
complete -c tunelo -n "__fish_seen_subcommand_from http-server" -l ip -d 'IP address to listen'
complete -c tunelo -n "__fish_seen_subcommand_from http-server" -l port -d 'Port number to listen'
complete -c tunelo -n "__fish_seen_subcommand_from http-server" -s h -l help -d 'Prints help information'
complete -c tunelo -n "__fish_seen_subcommand_from http-server" -s V -l version -d 'Prints version information'
complete -c tunelo -n "__fish_seen_subcommand_from help" -s h -l help -d 'Prints help information'
complete -c tunelo -n "__fish_seen_subcommand_from help" -s V -l version -d 'Prints version information'
