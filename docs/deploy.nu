#!/usr/bin/env nu

# Despliega la web de documentación estática en https://synapsers.app/
# excluyendo carpetas internas y metadatos (.synapse, etc.)
def main [] {
    let docs_dir = ($env.FILE_PWD | default $env.PWD)
    let temp_dist = (mktemp -d -t synapsers-docs-XXXXXX)

    print $"Preparando archivos limpios en ($temp_dist)..."
    cd $docs_dir
    fd -t f -d 1 -E "*.nu" | lines | each { |f| cp $f $temp_dist }

    let count = (ls $temp_dist | length)
    print $"Subiendo ($count) archivos a Cloudflare Workers..."

    bun x wrangler deploy --name synapsersapp --assets $temp_dist --compatibility-date 2026-11-12

    rm -rf $temp_dist
    print "Despliegue completado exitosamente en https://synapsers.app/ 🎉"
}
