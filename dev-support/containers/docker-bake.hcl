group "default" {
  targets = ["tunelo"]
}

target "tunelo" {
  dockerfile = "dev-support/containers/alpine/Containerfile"
  platforms  = ["linux/amd64"]
  target     = "final"
  contexts = {
    rust   = "docker-image://docker.io/library/rust:1.89.0-alpine3.22"
    alpine = "docker-image://docker.io/library/alpine:3.22"
  }
  labels = {
    "description"                     = "Container image for Tunelo"
    "image.type"                      = "final"
    "image.authors"                   = "46590321+xrelkd@users.noreply.github.com"
    "image.vendor"                    = "xrelkd"
    "image.description"               = "Tunelo - Proxy server that supports SOCKS4a, SOCKS5 and HTTP tunnel"
    "org.opencontainers.image.source" = "https://github.com/xrelkd/tunelo"
  }
}
