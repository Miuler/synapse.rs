Esta es la arquitectura de referencia definitiva para construir un motor de notas y enlaces estilo Obsidian en Rust, seleccionando exclusivamente los crates con cero coste innecesario de abstracción (*zero-cost abstractions*), paralelismo óptimo y layouts de memoria compactos:

### **1\. Ingesta y Monitoreo del Filesystem**

> * **jwalk:** Recorrido multihilo (*parallel directory walking*) del vault en el arranque en frío. Paraleliza el escaneo de inodos usando un pool de hilos (rayon), dejando obsoleto a walkdir monocanal en discos NVMe.  
> * **notify (v6+) \+ notify-debouncer-mini:** Suscripción reactiva nativa (inotify en Linux, kqueue, IOCP). El *debouncer* es mandatorio para coalescer eventos múltiples del editor (guardado rápido, auto-guardado atómico) y evitar ráfagas de reindexación inútiles.

### **2\. Tokenización y Parsing**

> * **pulldown-cmark:** El parser CommonMark más veloz del ecosistema. Su API basada en iteradores *pull* emite eventos (Start(Tag::Link), Start(Tag::Heading)) permitiendo extraer metadatos sin construir un AST completo en heap.  
> * **gray\_matter \+ serde\_yaml:** Extracción rápida de frontmatter (YAML/TOML/JSON) delimitado al inicio del archivo sin parsear el cuerpo de la nota.

### **3\. Índices en Memoria y Tipos de Datos (Zero-Alloc)**

> * **dashmap:** Tabla hash concurrente particionada por segmentos (*shards*). Reemplaza al clásico Arc\<RwLock\<HashMap\<K, V\>\>\>, eliminando la contención global entre el hilo indexador y los hilos lectores.  
> * **compact\_str:** Reemplazo directo para String en identificadores, slugs, tags y nombres de archivo. Implementa SSO (*Small String Optimization*) de hasta 24 bytes en el *stack* (64-bit), reduciendo drásticamente la presión sobre el asignador de memoria del sistema (*global allocator*).  
> * **petgraph:** Para mantener el grafo dirigido de adyacencias (DiGraphMap) entre notas. Resuelve cálculo de caminos mínimos, detección de ciclos y backlinks en microsegundos.

### **4\. Motores de Búsqueda y Resolución**

> * **nucleo / nucleo-matcher:** **Quick Switcher / Fuzzy Finder.** El motor del editor Helix. Utiliza prefiltrado SIMD sobre bytes ASCII, compresión de matriz dinámica en caché L1/L2 y concurrencia sin bloqueos para filtrar decenas de miles de rutas mientras el usuario escribe.  
> * **tantivy:** **Búsqueda Full-Text (FTS).** El estándar indiscutible. Algoritmo BM25, compresión con FST (*Finite State Transducers*), ejecución multihilo y soporte para índices respaldados por mmap o 100% en RAM.

### **5\. Runtime y Paralelismo**

> * **rayon:** Procesamiento en paralelo de la cola de archivos durante el arranque o reindexaciones masivas (*work-stealing threadpool*).  
> * **tokio:** Si el motor expone IPC/gRPC/WebSockets o sirve a una UI reactiva desacoplada.  
> * **mimalloc:** Asignador de memoria alternativo (\#\[global\_allocator\]). Reduce la fragmentación y supera al *allocator* del sistema cuando múltiples hilos crean y destruyen cadenas/metadatos concurrentemente.

### **Composición Arquitectónica del Stack**

| Capa | Crate / Tecnología | Propósito Técnico |
| :---- | :---- | :---- |
| **Arranque en frío** | jwalk \+ rayon | I/O paralelo a nivel de filesystem. |
| **Watcher reactivo** | notify \+ debouncer | Notificaciones del kernel coalescidas. |
| **Extracción AST** | pulldown-cmark | Parseo sin asignaciones intermedias. |
| **Caché de rutas/metadatos** | dashmap \+ compact\_str | Almacenamiento en RAM sin lock global ni overhead de heap. |
| **Grafo relacional** | petgraph | Resolución de backlinks y topología de red. |
| **Fuzzy Switcher (Ctrl+O)** | nucleo | Matcher con aceleración SIMD y scoring multihilo. |
| **FTS (Búsqueda en cuerpo)** | tantivy | Índice invertido BM25 con FST. |

