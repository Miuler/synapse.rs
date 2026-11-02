# Mediciones del refactor de arquitectura

Comando (modo release, contra una bóveda real; solo lee la bóveda):

```bash
SYNAPSE_BENCH_VAULT=/ruta/a/la/boveda cargo test --release --test baseline_measurements -- --ignored --nocapture
```

Bóveda: MisNotas (6 053 notas indexadas). Valores = mediana.

| Etapa | Abrir bóveda (frío) | Búsqueda rápida | Guardar nota 2 MB |
|---|---|---|---|
| 0 · línea base (main 3ac005a) | 139,1 ms | 13,8 ms | 1,29 ms |

Hallazgo de la etapa 0: en el Quick Open, los empates entre archivos con el mismo nombre y el
mismo puntaje (p. ej. `a/notas.md` y `b/notas.md`) se ordenan por el orden de iteración del
`DashMap`, que cambia entre ejecuciones. No se corrige en el refactor (cambiaría el
comportamiento); queda fuera de las pruebas de caracterización.
