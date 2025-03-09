use criterion::criterion_main;

mod vte;

criterion_main! {
    vte::vte_bench
}
