//! Ajustes del asignador de memoria del proceso.

/// Devuelve al sistema la memoria que el asignador ya liberó pero retiene.
///
/// Los recorridos de la bóveda al abrirla (reconciliación, sincronización del índice de texto
/// completo y escaneo del vigilante) corren en paralelo y reservan cientos de MB temporales. glibc
/// conserva esa memoria liberada en sus arenas: con ~6 000 notas el backend quedaba en ~570 MB
/// cuando lo vivo son ~200 MB. Llamarla al terminar esas tareas cuesta pocos ms.
pub fn release_freed_memory() {
    #[cfg(all(target_os = "linux", target_env = "gnu"))]
    {
        unsafe extern "C" {
            fn malloc_trim(pad: usize) -> i32;
        }
        // SAFETY: malloc_trim es seguro de llamar desde cualquier hilo en cualquier momento.
        unsafe {
            malloc_trim(0);
        }
    }
}
