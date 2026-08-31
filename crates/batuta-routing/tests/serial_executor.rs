//! La compuerta real impide dos invocaciones simultáneas incluso entre hilos.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};

use batuta_routing::SerialExecutionGate;

#[test]
fn el_ejecutor_instrumentado_nunca_observa_dos_rutas_activas() {
    let gate = Arc::new(SerialExecutionGate::new());
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let worker = {
        let gate = Arc::clone(&gate);
        let entered = Arc::clone(&entered);
        let release = Arc::clone(&release);
        let active = Arc::clone(&active);
        let maximum = Arc::clone(&maximum);
        std::thread::spawn(move || {
            gate.run(|| {
                let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                maximum.fetch_max(now, Ordering::SeqCst);
                entered.wait();
                release.wait();
                active.fetch_sub(1, Ordering::SeqCst);
            })
            .unwrap();
        })
    };
    entered.wait();
    assert!(gate.run(|| maximum.store(99, Ordering::SeqCst)).is_err());
    release.wait();
    worker.join().unwrap();

    assert_eq!(maximum.load(Ordering::SeqCst), 1);
}
