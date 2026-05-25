#![allow(deprecated)]
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn benchmark_clone(c: &mut Criterion) {
    // Attempting to bench the difference
    c.bench_function("clone_arc_option", |b| {
        use std::sync::Arc;
        let opt = Some(Arc::new(42));
        b.iter(|| {
            let cloned = black_box(opt.clone());
            if let Some(mut _val) = cloned {
                black_box(_val);
            }
        });
    });

    c.bench_function("borrow_arc_option", |b| {
        use std::sync::Arc;
        let mut opt = Some(Arc::new(42));
        b.iter(|| {
            if let Some(val) = black_box(&mut opt) {
                black_box(val);
            }
        });
    });
}

criterion_group!(benches, benchmark_clone);
criterion_main!(benches);
