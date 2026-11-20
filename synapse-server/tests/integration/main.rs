//! Pruebas de integración como módulos de un solo binario (evita un enlazado completo por archivo).
//! Un grupo se corre con `cargo test --test integration <módulo>::`.

mod architecture;
mod grpc;
mod health;
mod support;
