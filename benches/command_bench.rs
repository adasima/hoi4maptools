use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::hint::black_box;

#[path = "../src/core/mod.rs"]
mod core;
#[path = "../src/map/mod.rs"]
mod map;
#[path = "../src/painter/mod.rs"]
mod painter;

use core::command::Command;
use map::command::FillCommand;
use map::definition::DefinitionTable;
use map::graph::{ProvinceColor, ProvinceGraph, ProvinceId};
use map::ProjectState;

fn bench_fill_command(c: &mut Criterion) {
    let width = 2000;
    let height = 1000;
    let mut pixels = vec![0u8; (width * height * 3) as usize];

    // Fill most with white
    for i in 0..(width * height) as usize {
        pixels[i * 3] = 255;
        pixels[i * 3 + 1] = 255;
        pixels[i * 3 + 2] = 255;
    }

    // Put a small block of red
    for y in 100..200 {
        for x in 100..200 {
            let idx = ((y * width + x) * 3) as usize;
            pixels[idx] = 255;
            pixels[idx + 1] = 0;
            pixels[idx + 2] = 0;
        }
    }

    let mut color_id_map: HashMap<u32, ProvinceId> = HashMap::new();
    color_id_map.insert(ProvinceColor::new(255, 255, 255).to_key(), 1);
    color_id_map.insert(ProvinceColor::new(255, 0, 0).to_key(), 2);
    color_id_map.insert(ProvinceColor::new(0, 255, 0).to_key(), 3);

    let graph = ProvinceGraph::build_from_pixels(&pixels, width, height, &color_id_map);

    let mut project = ProjectState {
        pixels: pixels.clone(),
        width,
        height,
        definitions: DefinitionTable::new(),
        graph,
        project_dir: std::path::PathBuf::new(),
        dirty_rect: None,
    };

    let mut cmd = FillCommand {
        from_color: ProvinceColor::new(255, 0, 0),
        to_color: ProvinceColor::new(0, 255, 0),
    };

    c.bench_function("fill_command_execute", |b| {
        b.iter(|| {
            cmd.execute(black_box(&mut project)).unwrap();
            cmd.undo(black_box(&mut project)).unwrap();
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10);
    targets = bench_fill_command
}
criterion_main!(benches);

// This runs a simple test that will print the duration
#[test]
fn test_bench_duration() {
    use std::collections::HashMap;
    use std::time::Instant;

    let width = 2000;
    let height = 1000;
    let mut pixels = vec![0u8; (width * height * 3) as usize];

    for i in 0..(width * height) as usize {
        pixels[i * 3] = 255;
        pixels[i * 3 + 1] = 255;
        pixels[i * 3 + 2] = 255;
    }

    for y in 100..200 {
        for x in 100..200 {
            let idx = ((y * width + x) * 3) as usize;
            pixels[idx] = 255;
            pixels[idx + 1] = 0;
            pixels[idx + 2] = 0;
        }
    }

    let mut color_id_map: HashMap<u32, ProvinceId> = HashMap::new();
    color_id_map.insert(ProvinceColor::new(255, 255, 255).to_key(), 1);
    color_id_map.insert(ProvinceColor::new(255, 0, 0).to_key(), 2);
    color_id_map.insert(ProvinceColor::new(0, 255, 0).to_key(), 3);

    let graph = ProvinceGraph::build_from_pixels(&pixels, width, height, &color_id_map);

    let mut project = ProjectState {
        pixels: pixels.clone(),
        width,
        height,
        definitions: DefinitionTable::new(),
        graph,
        project_dir: std::path::PathBuf::new(),
        dirty_rect: None,
    };

    let mut cmd = FillCommand {
        from_color: ProvinceColor::new(255, 0, 0),
        to_color: ProvinceColor::new(0, 255, 0),
    };

    let start = Instant::now();
    for _ in 0..10 {
        cmd.execute(&mut project).unwrap();
        cmd.undo(&mut project).unwrap();
    }
    let duration = start.elapsed();
    println!("Baseline duration for 10 fills: {:?}", duration);
}
