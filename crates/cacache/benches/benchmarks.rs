use async_std::{fs as afs, task};
use std::fs::{self, File};
use std::io::prelude::*;

use cacache;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile;

const NUM_REPEATS: usize = 10;

fn baseline_read_sync(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("test_file");
    let data = b"hello world";
    let mut fd = File::create(&path).unwrap();
    fd.write_all(data).unwrap();
    drop(fd);
    c.bench_function("baseline_read_sync", move |b| {
        b.iter(|| fs::read(&path).unwrap())
    });
}

fn baseline_read_many_sync(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let paths: Vec<_> = (0..)
        .take(NUM_REPEATS)
        .map(|i| tmp.path().join(format!("test_file_{}", i)))
        .collect();
    let data = b"hello world";
    for path in paths.iter() {
        let mut fd = File::create(&path).unwrap();
        fd.write_all(data).unwrap();
        drop(fd);
    }
    c.bench_function("baseline_read_many_sync", move |b| {
        b.iter(|| {
            for path in paths.iter() {
                fs::read(black_box(&path)).unwrap();
            }
        })
    });
}

fn baseline_read_async(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("test_file");
    let data = b"hello world";
    let mut fd = File::create(&path).unwrap();
    fd.write_all(data).unwrap();
    drop(fd);
    c.bench_function("baseline_read_async", move |b| {
        b.iter(|| task::block_on(afs::read(&path)))
    });
}

fn baseline_read_many_async(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let paths: Vec<_> = (0..)
        .take(NUM_REPEATS)
        .map(|i| tmp.path().join(format!("test_file_{}", i)))
        .collect();
    let data = b"hello world";
    for path in paths.iter() {
        let mut fd = File::create(&path).unwrap();
        fd.write_all(data).unwrap();
        drop(fd);
    }
    c.bench_function("baseline_read_many_async", move |b| {
        b.iter(|| {
            let tasks = paths.iter().map(|path| afs::read(black_box(path)));
            task::block_on(futures::future::join_all(tasks));
        })
    });
}

fn read_hash_sync(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data = b"hello world".to_vec();
    let sri = cacache::write_sync(&cache, "hello", data).unwrap();
    c.bench_function("get::data_hash_sync", move |b| {
        b.iter(|| cacache::read_hash_sync(black_box(&cache), black_box(&sri)).unwrap())
    });
}

fn read_hash_many_sync(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data: Vec<_> = (0..)
        .take(NUM_REPEATS)
        .map(|i| format!("test_file_{}", i))
        .collect();
    let sris: Vec<_> = data
        .iter()
        .map(|datum| cacache::write_sync(&cache, "hello", datum).unwrap())
        .collect();
    c.bench_function("get::data_hash_many_sync", move |b| {
        b.iter(|| {
            for sri in sris.iter() {
                cacache::read_hash_sync(black_box(&cache), black_box(&sri)).unwrap();
            }
        })
    });
}

fn read_sync(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data = b"hello world".to_vec();
    cacache::write_sync(&cache, "hello", data).unwrap();
    c.bench_function("get::data_sync", move |b| {
        b.iter(|| cacache::read_sync(black_box(&cache), black_box(String::from("hello"))).unwrap())
    });
}

fn read_hash_sync_big_data(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data = vec![1; 1024 * 1024 * 5];
    let sri = cacache::write_sync(&cache, "hello", data).unwrap();
    c.bench_function("get_hash_big_data", move |b| {
        b.iter(|| cacache::read_hash_sync(black_box(&cache), black_box(&sri)).unwrap())
    });
}

fn read_hash_many_async(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data: Vec<_> = (0..)
        .take(NUM_REPEATS)
        .map(|i| format!("test_file_{}", i))
        .collect();
    let sris: Vec<_> = data
        .iter()
        .map(|datum| cacache::write_sync(&cache, "hello", datum).unwrap())
        .collect();
    c.bench_function("get::data_hash_many", move |b| {
        b.iter(|| {
            let tasks = sris
                .iter()
                .map(|sri| cacache::read_hash(black_box(&cache), black_box(&sri)));
            task::block_on(futures::future::join_all(tasks));
        })
    });
}

fn read_hash_async(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data = b"hello world".to_vec();
    let sri = cacache::write_sync(&cache, "hello", data).unwrap();
    c.bench_function("get::data_hash", move |b| {
        b.iter(|| task::block_on(cacache::read_hash(black_box(&cache), black_box(&sri))).unwrap())
    });
}

fn read_async(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data = b"hello world".to_vec();
    cacache::write_sync(&cache, "hello", data).unwrap();
    c.bench_function("get::data", move |b| {
        b.iter(|| task::block_on(cacache::read(black_box(&cache), black_box("hello"))).unwrap())
    });
}

fn read_hash_async_big_data(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    let data = vec![1; 1024 * 1024 * 5];
    let sri = cacache::write_sync(&cache, "hello", data).unwrap();
    c.bench_function("get::data_big_data", move |b| {
        b.iter(|| task::block_on(cacache::read_hash(black_box(&cache), black_box(&sri))).unwrap())
    });
}

fn write_hash_async(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let cache = tmp.path().to_owned();
    c.bench_function("put::data", move |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            for i in 0..iters {
                task::block_on(cacache::write_hash(&cache, format!("hello world{}", i))).unwrap();
            }
            start.elapsed()
        })
    });
}

criterion_group!(
    benches,
    baseline_read_sync,
    baseline_read_many_sync,
    baseline_read_async,
    baseline_read_many_async,
    read_hash_async,
    read_hash_many_async,
    read_async,
    write_hash_async,
    read_hash_sync,
    read_hash_many_sync,
    read_sync,
    read_hash_async_big_data,
    read_hash_sync_big_data
);
criterion_main!(benches);
