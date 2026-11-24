#!/usr/bin/env nu

# Genera la web (inglés y español) y la publica en https://synapsers.app/
def main [] {
    cd $env.FILE_PWD
    bun build.ts
    bun x wrangler deploy --name synapsersapp --assets dist --compatibility-date 2026-11-12
    print "Despliegue completado exitosamente en https://synapsers.app/ 🎉"
}
