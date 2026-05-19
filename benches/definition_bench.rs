use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

#[path = "../src/map/definition.rs"]
mod definition;
#[path = "../src/map/graph.rs"]
mod graph;

use definition::{DefinitionTable, ProvinceType};
use graph::{ProvinceColor, ProvinceId};

fn bench_get(c: &mut Criterion) {
    let mut table = DefinitionTable::new();

    // Create a large table to make O(N) noticeable
    for i in 1..=20000 {
        let color = ProvinceColor::new(
            (i % 256) as u8,
            ((i / 256) % 256) as u8,
            ((i / 65536) % 256) as u8,
        );
        table.add_province(color, ProvinceType::Land, "plains", 1);
    }

    c.bench_function("definition_get", |b| {
        b.iter(|| {
            // Query some middle and end IDs
            black_box(table.get(10000));
            black_box(table.get(19999));
        });
    });

    c.bench_function("definition_get_mut", |b| {
        b.iter(|| {
            // Query some middle and end IDs
            black_box(table.get_mut(10000));
            black_box(table.get_mut(19999));
        });
    });
}

criterion_group!(benches, bench_get);
criterion_main!(benches);
