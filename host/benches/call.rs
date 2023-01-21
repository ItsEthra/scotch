use criterion::{criterion_group, criterion_main, Criterion};
use scotch_host::{guest_functions, make_exports, WasmPlugin};
use std::{hint::black_box, iter::successors};

guest_functions! {
    pub add_up_list: fn(list: &Vec<i32>) -> i32;
}

fn call(plugin: &WasmPlugin, numbers: &Vec<i32>, result: i32) {
    assert_eq!(
        black_box(plugin.function::<add_up_list>()(numbers)).unwrap(),
        result
    );
}

fn bench_call(c: &mut Criterion) {
    let plugin = WasmPlugin::builder()
        .with_env(())
        .from_binary(include_bytes!("../../examples/runner/plugin.wasm"))
        .unwrap()
        .with_exports(make_exports!(add_up_list))
        .finish()
        .unwrap();

    let numbers: Vec<i32> = successors(Some(1), |n| Some(n * 2)).take(1000).collect();
    let result: i32 = numbers.iter().sum();

    c.bench_function("sum 1000 vec<i32>", |b| {
        b.iter(|| call(&plugin, &numbers, result))
    });
}

criterion_group!(benches, bench_call);
criterion_main!(benches);
