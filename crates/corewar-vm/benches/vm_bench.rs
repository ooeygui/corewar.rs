use criterion::{black_box, criterion_group, criterion_main, Criterion};
use corewar_vm::{Battle, VmConfig};

fn bench_empty_battle(c: &mut Criterion) {
    c.bench_function("empty_battle_8000", |b| {
        b.iter(|| {
            let config = VmConfig::default();
            let mut battle = Battle::new(black_box(config));
            battle.run()
        });
    });
}

criterion_group!(benches, bench_empty_battle);
criterion_main!(benches);
