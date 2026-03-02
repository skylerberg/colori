use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use colori_core::unordered_cards::UnorderedCards;
use rand::rngs::SmallRng;
use rand::SeedableRng;

fn make_set_50() -> UnorderedCards {
    let mut set = UnorderedCards::new();
    for i in 0..50u8 {
        set.insert(i);
    }
    set
}

fn benchmarks(c: &mut Criterion) {
    c.bench_function("bench_insert", |b| {
        b.iter(|| {
            let mut set = UnorderedCards::new();
            for i in 0..50u8 {
                set.insert(black_box(i));
            }
            set
        });
    });

    c.bench_function("bench_contains", |b| {
        let set = make_set_50();
        b.iter(|| {
            for i in 0..50u8 {
                black_box(set.contains(black_box(i)));
            }
        });
    });

    c.bench_function("bench_len", |b| {
        let set = make_set_50();
        b.iter(|| black_box(set.len()));
    });

    c.bench_function("bench_union", |b| {
        let a = make_set_50();
        let mut b_set = UnorderedCards::new();
        for i in 50..100u8 {
            b_set.insert(i);
        }
        b.iter(|| black_box(black_box(a).union(black_box(b_set))));
    });

    c.bench_function("bench_intersection", |b| {
        let a = make_set_50();
        let mut b_set = UnorderedCards::new();
        for i in 25..75u8 {
            b_set.insert(i);
        }
        b.iter(|| black_box(black_box(a).intersection(black_box(b_set))));
    });

    c.bench_function("bench_difference", |b| {
        let a = make_set_50();
        let mut b_set = UnorderedCards::new();
        for i in 25..75u8 {
            b_set.insert(i);
        }
        b.iter(|| black_box(black_box(a).difference(black_box(b_set))));
    });

    c.bench_function("bench_iter_collect", |b| {
        let set = make_set_50();
        b.iter(|| {
            let v: Vec<u8> = black_box(set).iter().collect();
            black_box(v)
        });
    });

    c.bench_function("bench_pick_random", |b| {
        let set = make_set_50();
        let mut rng = SmallRng::seed_from_u64(42);
        b.iter(|| black_box(set.pick_random(&mut rng)));
    });

    c.bench_function("bench_draw", |b| {
        let mut rng = SmallRng::seed_from_u64(42);
        b.iter_batched(
            || make_set_50(),
            |mut set| {
                black_box(set.draw(&mut rng))
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("bench_draw_multiple_5_of_50", |b| {
        let mut rng = SmallRng::seed_from_u64(42);
        b.iter_batched(
            || make_set_50(),
            |mut set| {
                black_box(set.draw_multiple(black_box(5), &mut rng))
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("bench_draw_multiple_9_of_50", |b| {
        let mut rng = SmallRng::seed_from_u64(42);
        b.iter_batched(
            || make_set_50(),
            |mut set| {
                black_box(set.draw_multiple(black_box(9), &mut rng))
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("bench_draw_up_to_5_of_50", |b| {
        let mut rng = SmallRng::seed_from_u64(42);
        b.iter_batched(
            || make_set_50(),
            |mut set| {
                black_box(set.draw_up_to(black_box(5), &mut rng))
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
