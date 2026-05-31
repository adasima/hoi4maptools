use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn benchmark_hex_points(c: &mut Criterion) {
    let poly = vec![
        (0.0, 0.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.0, 1.0),
        (-1.0, 0.5),
        (-0.5, -0.5),
    ];

    c.bench_function("with_clone", |b| {
        b.iter(|| {
            let mut points_screen = Vec::with_capacity(6);
            for mp in poly.iter() {
                points_screen.push(*mp);
            }
            let mut lines = points_screen.clone();
            if let Some(first) = lines.first().copied() {
                lines.push(first);
            }
            black_box(lines);
        });
    });

    c.bench_function("without_clone", |b| {
        b.iter(|| {
            let mut points_screen = Vec::with_capacity(7);
            for mp in poly.iter() {
                points_screen.push(*mp);
            }
            if let Some(first) = points_screen.first().copied() {
                points_screen.push(first);
            }
            black_box(points_screen);
        });
    });
}

criterion_group!(benches, benchmark_hex_points);
criterion_main!(benches);
